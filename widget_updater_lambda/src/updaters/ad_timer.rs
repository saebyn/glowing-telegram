use aws_sdk_secretsmanager::Client as SecretsManagerClient;
use chrono::{DateTime, Duration, Utc};
use gt_secrets::UserSecretPathProvider;
use serde::Deserialize;
use serde_json::json;
use tracing::{instrument, warn};
use types::TwitchSessionSecret;

use super::WidgetUpdate;

/// Minimal response from the Twitch OAuth /validate endpoint.
#[derive(Deserialize)]
struct TwitchValidateResponse {
    user_id: String,
}

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

    // Return None if nothing changed to avoid unnecessary DynamoDB writes.
    if new_next_ad_at == current_state.next_ad_at
        && ad_data.snooze_count == current_state.snooze_count
        && new_snoozed_at == current_state.snoozed_at
        && new_back_from_ads_until == current_state.back_from_ads_until
    {
        return None;
    }

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

    // Derive the Twitch broadcaster_id from the access token by calling the
    // Twitch OAuth /validate endpoint, rather than assuming widget.user_id is
    // a Twitch user ID (it is a Cognito sub elsewhere in the codebase).
    let validate_resp = http
        .get("https://id.twitch.tv/oauth2/validate")
        .header("Authorization", format!("OAuth {}", access_token))
        .send()
        .await
        .map_err(|e| format!("Failed to validate Twitch token: {e}"))?;

    if !validate_resp.status().is_success() {
        return Err(format!(
            "Twitch token validation failed with status {}",
            validate_resp.status()
        ));
    }

    let validate_body: TwitchValidateResponse = validate_resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Twitch validate response: {e}"))?;

    let broadcaster_id = validate_body.user_id;

    let ad_data = fetch_ad_schedule(
        http,
        &broadcaster_id,
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_config(back_from_ads_duration_ms: i64) -> AdTimerConfig {
        AdTimerConfig {
            back_from_ads_duration_ms,
        }
    }

    fn make_state(
        next_ad_at: Option<&str>,
        snooze_count: i64,
        snoozed_at: Option<&str>,
        back_from_ads_until: Option<&str>,
    ) -> AdTimerState {
        AdTimerState {
            next_ad_at: next_ad_at.map(String::from),
            snooze_count,
            snoozed_at: snoozed_at.map(String::from),
            back_from_ads_until: back_from_ads_until.map(String::from),
        }
    }

    fn make_ad_data(
        next_ad_at: i64,
        snooze_count: i64,
        last_ad_at: i64,
    ) -> AdScheduleData {
        AdScheduleData {
            next_ad_at,
            snooze_count,
            last_ad_at,
        }
    }

    #[test]
    fn test_next_ad_at_set_when_nonzero() {
        let now = Utc::now();
        let future = now + Duration::minutes(10);
        let ad_data = make_ad_data(future.timestamp(), 3, 0);
        let current = make_state(None, 3, None, None);
        let config = make_config(10_000);

        let result = compute_new_state(now, &ad_data, &current, &config);
        assert!(result.is_some());
        let state = result.unwrap();
        assert!(!state["nextAdAt"].is_null());
    }

    #[test]
    fn test_next_ad_at_cleared_when_zero() {
        let now = Utc::now();
        let ad_data = make_ad_data(0, 3, 0);
        // Previously had a non-null nextAdAt
        let prev_next = (now + Duration::minutes(10)).to_rfc3339();
        let current = make_state(Some(&prev_next), 3, None, None);
        let config = make_config(10_000);

        let result = compute_new_state(now, &ad_data, &current, &config);
        assert!(result.is_some());
        let state = result.unwrap();
        assert!(state["nextAdAt"].is_null());
    }

    #[test]
    fn test_snooze_detected_when_count_decrements() {
        let now = Utc::now();
        let future = (now + Duration::minutes(10)).timestamp();
        let ad_data = make_ad_data(future, 2, 0); // was 3, now 2
        let current = make_state(None, 3, None, None);
        let config = make_config(10_000);

        let result = compute_new_state(now, &ad_data, &current, &config);
        assert!(result.is_some());
        let state = result.unwrap();
        assert!(!state["snoozedAt"].is_null());
        assert_eq!(state["snoozeCount"], 2);
    }

    #[test]
    fn test_snooze_not_detected_when_count_unchanged() {
        let now = Utc::now();
        let future = (now + Duration::minutes(10)).timestamp();
        let ad_data = make_ad_data(future, 3, 0);
        let current = make_state(None, 3, None, None);
        let config = make_config(10_000);

        let result = compute_new_state(now, &ad_data, &current, &config);
        // next_ad_at changed (None -> Some), so it's not None
        let state = result.unwrap();
        assert!(state["snoozedAt"].is_null());
    }

    #[test]
    fn test_back_from_ads_until_set_after_break() {
        let now = Utc::now();
        // Ad was scheduled in the past (we were in a break)
        let past_next_ad = (now - Duration::minutes(1)).to_rfc3339();
        // New next ad is in the future (break is over)
        let new_next_ad = (now + Duration::minutes(30)).timestamp();
        let ad_data = make_ad_data(new_next_ad, 3, now.timestamp());
        let current = make_state(Some(&past_next_ad), 3, None, None);
        let config = make_config(10_000);

        let result = compute_new_state(now, &ad_data, &current, &config);
        assert!(result.is_some());
        let state = result.unwrap();
        assert!(!state["backFromAdsUntil"].is_null());
    }

    #[test]
    fn test_back_from_ads_until_not_set_when_not_in_break() {
        let now = Utc::now();
        // nextAdAt is in the future (not in an ad break)
        let future_next_ad = (now + Duration::minutes(5)).to_rfc3339();
        let new_next_ad = (now + Duration::minutes(30)).timestamp();
        let ad_data = make_ad_data(new_next_ad, 3, 0);
        let current = make_state(Some(&future_next_ad), 3, None, None);
        let config = make_config(10_000);

        let result = compute_new_state(now, &ad_data, &current, &config);
        // The state changed (nextAdAt moved forward), so Some is returned
        let state = result.unwrap();
        assert!(state["backFromAdsUntil"].is_null());
    }

    #[test]
    fn test_no_update_when_state_unchanged() {
        // Use a fixed past time so there's no sub-second rounding issue.
        let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        // Build next_ad_at from an integer timestamp so the round-trip is lossless.
        let next_ad_ts = now.timestamp() + 600; // 10 minutes from now
        let next_ad_rfc3339 = DateTime::from_timestamp(next_ad_ts, 0)
            .unwrap()
            .to_rfc3339();
        let ad_data = make_ad_data(next_ad_ts, 3, 0);
        let current = make_state(Some(&next_ad_rfc3339), 3, None, None);
        let config = make_config(10_000);

        // Nothing has changed: the timestamp round-trips exactly, snooze count is
        // the same, and no ad break is in progress. Should return None.
        let result = compute_new_state(now, &ad_data, &current, &config);
        assert!(result.is_none(), "expected None when state is unchanged");
    }
}
