use aws_sdk_secretsmanager::Client as SecretsManagerClient;
use chrono::{DateTime, Duration, Utc};
use gt_secrets::UserSecretPathProvider;
use serde::Deserialize;
use serde_json::json;
use tracing::{instrument, warn};
use types::TwitchSessionSecret;

use super::WidgetUpdate;

/// Configuration stored per widget instance in DynamoDB.
#[derive(Debug, Deserialize)]
struct AdTimerConfig {
    /// Milliseconds to show the "Back from Ads" message after an ad break ends.
    #[serde(
        rename = "backFromAdsDuration",
        default = "default_back_from_ads_duration_ms"
    )]
    back_from_ads_duration_ms: i64,
}

fn default_back_from_ads_duration_ms() -> i64 {
    10_000
}

/// State stored per widget instance in DynamoDB (the fields the backend owns).
#[derive(Debug, Deserialize)]
struct AdTimerState {
    /// ISO 8601 timestamp of next scheduled ad, or null.
    #[serde(rename = "nextAdAt")]
    next_ad_at: Option<String>,
    /// Current snooze count from the last Twitch poll.
    #[serde(rename = "snoozeCount", default)]
    snooze_count: i64,
    /// ISO 8601 timestamp when a snooze was detected, or null.
    #[serde(rename = "snoozedAt")]
    snoozed_at: Option<String>,
    /// ISO 8601 timestamp until when to show "Back from Ads", or null.
    #[serde(rename = "backFromAdsUntil")]
    back_from_ads_until: Option<String>,
}

/// Response from GET /helix/channels/ads
#[derive(Debug, Deserialize)]
struct AdScheduleData {
    /// Unix timestamp (seconds) of next scheduled ad. 0 means no ad scheduled.
    next_ad_at: i64,
    /// Number of available snoozes remaining.
    snooze_count: i64,
    /// Unix timestamp (seconds) of last ad break. 0 means no previous ad.
    last_ad_at: i64,
}

#[derive(Debug, Deserialize)]
struct AdScheduleResponse {
    data: Vec<AdScheduleData>,
}

/// Fetch the ad schedule for a broadcaster from the Twitch API.
async fn fetch_ad_schedule(
    http: &reqwest::Client,
    broadcaster_id: &str,
    access_token: &str,
    twitch_client_id: &str,
) -> Result<AdScheduleData, String> {
    let url = format!(
        "https://api.twitch.tv/helix/channels/ads?broadcaster_id={}",
        broadcaster_id
    );

    let response = http
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Client-Id", twitch_client_id)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Twitch API returned status {}",
            response.status()
        ));
    }

    let body: AdScheduleResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {e}"))?;

    body.data
        .into_iter()
        .next()
        .ok_or_else(|| "Empty data array in ad schedule response".to_string())
}

/// Compute the new state for a single ad_timer widget.
///
/// Returns `None` if the state is unchanged (no update needed).
#[allow(clippy::too_many_arguments)]
fn compute_new_state(
    now: DateTime<Utc>,
    ad_data: &AdScheduleData,
    current_state: &AdTimerState,
    config: &AdTimerConfig,
) -> Option<serde_json::Value> {
    let new_next_ad_at: Option<String> = if ad_data.next_ad_at > 0 {
        Some(
            DateTime::from_timestamp(ad_data.next_ad_at, 0)
                .unwrap_or(now)
                .to_rfc3339(),
        )
    } else {
        None
    };

    // Detect snooze: snooze_count decremented means user used a snooze.
    let new_snoozed_at = if ad_data.snooze_count < current_state.snooze_count {
        Some(now.to_rfc3339())
    } else {
        current_state.snoozed_at.clone()
    };

    // Detect ad completion: last_ad_at changed to a new non-zero value while
    // we were previously in (or past) an ad break (i.e., next_ad_at was in the past).
    let was_in_ad_break = current_state
        .next_ad_at
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|next| next.with_timezone(&Utc) <= now)
        .unwrap_or(false);

    let last_ad_changed = ad_data.last_ad_at > 0
        && {
            // Compare to what we previously recorded — we track this via nextAdAt being in the past.
            // A simpler heuristic: if we were in an ad break and now next_ad_at moved forward.
            let prev_next_ad = current_state
                .next_ad_at
                .as_deref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            let new_next_ad = new_next_ad_at
                .as_deref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            matches!((prev_next_ad, new_next_ad), (Some(prev), Some(next)) if next > prev)
        };

    let new_back_from_ads_until = if was_in_ad_break && last_ad_changed {
        let until =
            now + Duration::milliseconds(config.back_from_ads_duration_ms);
        Some(until.to_rfc3339())
    } else {
        current_state.back_from_ads_until.clone()
    };

    Some(json!({
        "nextAdAt": new_next_ad_at,
        "snoozeCount": ad_data.snooze_count,
        "snoozedAt": new_snoozed_at,
        "backFromAdsUntil": new_back_from_ads_until,
    }))
}

/// Compute state updates for all active ad_timer widgets.
///
/// Each widget belongs to a user; we fetch that user's Twitch token from
/// Secrets Manager, then call the Twitch ad schedule API on their behalf.
#[instrument(skip_all)]
pub async fn compute_ad_timer_updates(
    widgets: &[types::StreamWidget],
    secrets_manager: &SecretsManagerClient,
    http: &reqwest::Client,
    user_secret_path: &UserSecretPathProvider,
    twitch_client_id: &str,
) -> Vec<WidgetUpdate> {
    let now = Utc::now();
    let mut updates = Vec::new();

    for widget in widgets {
        let result = process_widget(
            widget,
            secrets_manager,
            http,
            user_secret_path,
            twitch_client_id,
            now,
        )
        .await;

        match result {
            Ok(Some(update)) => updates.push(update),
            Ok(None) => {}
            Err(e) => warn!("Skipping widget {}: {}", widget.id, e),
        }
    }

    updates
}

async fn process_widget(
    widget: &types::StreamWidget,
    secrets_manager: &SecretsManagerClient,
    http: &reqwest::Client,
    user_secret_path: &UserSecretPathProvider,
    twitch_client_id: &str,
    now: DateTime<Utc>,
) -> Result<Option<WidgetUpdate>, String> {
    let secret_id = user_secret_path.secret_path(&widget.user_id);

    let twitch_secret: TwitchSessionSecret =
        gt_secrets::get(secrets_manager, &secret_id)
            .await
            .map_err(|e| format!("Failed to get Twitch secret: {e}"))?;

    let access_token = twitch_secret
        .access_token
        .as_deref()
        .ok_or("No Twitch access token for user")?;

    // The broadcaster_id is the user_id in the Twitch token validation response.
    // It is stored in Secrets Manager alongside the access token.  We derive it
    // by calling the Twitch /validate endpoint — but to avoid an extra round-trip
    // on every poll we instead read it from the broadcaster_id field in the token
    // secret. The twitch_lambda stores it implicitly as the Cognito user_id maps
    // to the Twitch user. For now we pass user_id as the broadcaster ID; callers
    // that set up the widget must ensure user_id == Twitch broadcaster_id.
    //
    // A cleaner future improvement: store broadcaster_id in the widget record.
    let broadcaster_id = &widget.user_id;

    let ad_data = fetch_ad_schedule(
        http,
        broadcaster_id,
        access_token,
        twitch_client_id,
    )
    .await?;

    // Parse current widget state
    let current_state: AdTimerState = widget
        .state
        .as_ref()
        .and_then(|s| serde_json::to_value(s).ok())
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or(AdTimerState {
            next_ad_at: None,
            snooze_count: 0,
            snoozed_at: None,
            back_from_ads_until: None,
        });

    // Parse config (use defaults if missing)
    let config: AdTimerConfig = widget
        .config
        .as_ref()
        .and_then(|c| serde_json::to_value(c).ok())
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or(AdTimerConfig {
            back_from_ads_duration_ms: default_back_from_ads_duration_ms(),
        });

    let new_state_json =
        match compute_new_state(now, &ad_data, &current_state, &config) {
            Some(v) => v,
            None => return Ok(None),
        };

    // Convert JSON object into the HashMap<String, Option<JsonValue>> format the writer expects
    let state_map = new_state_json
        .as_object()
        .ok_or("State is not an object")?
        .iter()
        .map(|(k, v)| {
            (k.clone(), if v.is_null() { None } else { Some(v.clone()) })
        })
        .collect();

    Ok(Some(WidgetUpdate {
        id: widget.id.clone(),
        state: state_map,
    }))
}
