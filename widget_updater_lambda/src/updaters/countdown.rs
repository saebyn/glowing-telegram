use super::{WidgetUpdate, WidgetUpdater};
use chrono::{DateTime, Utc};
use serde_json::json;
use tracing::warn;

/// Updater for countdown timer widgets
///
/// Countdown widgets maintain:
/// - `enabled`: bool - whether countdown is actively ticking
/// - `duration_left`: i64 - seconds remaining (can be 0 if countdown finished)
/// - `last_tick_timestamp`: string (ISO 8601) - last time countdown was updated
///
/// Behavior:
/// - Ticks down by elapsed seconds when enabled=true
/// - Stops at 0 but stays enabled (to represent "countdown finished" state)
/// - Skips widgets that haven't been enabled or don't have valid state
pub struct CountdownUpdater;

impl WidgetUpdater for CountdownUpdater {
    fn compute_batch_updates(
        &self,
        widgets: &[crate::StreamWidget],
    ) -> Vec<WidgetUpdate> {
        let now = Utc::now();

        widgets
            .iter()
            .filter_map(|widget| {
                let state = widget.state.as_ref()?;

                // Check if countdown is enabled
                let enabled = state
                    .get("enabled")
                    .unwrap_or(&Some(json!(false)))
                    .clone()
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if !enabled {
                    return None; // Skip disabled countdowns
                }

                // Get last tick timestamp
                let last_tick = state
                    .get("last_tick_timestamp")
                    .unwrap_or(&Some(json!(null)))
                    .as_ref()
                    .and_then(|v| v.as_str())
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                let last_tick = match last_tick {
                    Some(dt) => dt,
                    None => {
                        warn!(
                            "Widget {} has enabled=true but no valid last_tick_timestamp",
                            widget.id
                        );
                        return None;
                    }
                };

                // Calculate elapsed seconds since last tick
                let elapsed_seconds = (now - last_tick).num_seconds();

                if elapsed_seconds < 1 {
                    return None; // Not time to tick yet
                }

                // Get current duration left
                let duration_left = state
                    .get("duration_left")
                    .unwrap_or(&Some(json!(0)))
                    .clone()
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);

                // Subtract elapsed time, but don't go below 0
                let new_duration_left = (duration_left - elapsed_seconds).max(0);

                // Build updated state
                let mut new_state = state.clone();
                new_state.insert("duration_left".to_string(), Some(json!(new_duration_left)));
                new_state.insert("last_tick_timestamp".to_string(), Some(json!(now.to_rfc3339())));

                // Note: We keep enabled=true even at 0, so frontend can show "finished" state
                // User can explicitly disable via action if they want to hide the widget

                Some(WidgetUpdate {
                    id: widget.id.clone(),
                    state: new_state,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use chrono::Duration;

    fn create_test_widget(
        id: &str,
        enabled: bool,
        duration_left: i64,
        last_tick: DateTime<Utc>,
    ) -> crate::StreamWidget {
        let mut state = HashMap::new();
        state.insert("enabled".to_string(), Some(json!(enabled)));
        state.insert("duration_left".to_string(), Some(json!(duration_left)));
        state.insert(
            "last_tick_timestamp".to_string(),
            Some(json!(last_tick.to_rfc3339())),
        );

        crate::StreamWidget {
            id: id.to_string(),
            stream_widget_type: types::StreamWidgetType::Countdown,
            active: Some(true),
            config: None,
            state: Some(state),

            access_token: None,
            user_id: "".to_string(),
            created_at: None,
            updated_at: None,
            title: "".to_string(),
        }
    }

    #[test]
    fn test_countdown_ticks_down() {
        let updater = CountdownUpdater;
        let last_tick = Utc::now() - Duration::seconds(5);
        let widget = create_test_widget("test-1", true, 60, last_tick);

        let updates = updater.compute_batch_updates(&[widget]);

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].id, "test-1");

        let duration = updates[0]
            .state
            .get("duration_left")
            .unwrap()
            .clone()
            .unwrap()
            .as_i64()
            .unwrap();
        assert!(duration >= 54 && duration <= 56); // Account for test timing variance
    }

    #[test]
    fn test_countdown_stops_at_zero() {
        let updater = CountdownUpdater;
        let last_tick = Utc::now() - Duration::seconds(10);
        let widget = create_test_widget("test-2", true, 5, last_tick);

        let updates = updater.compute_batch_updates(&[widget]);

        assert_eq!(updates.len(), 1);
        let duration = updates[0]
            .state
            .get("duration_left")
            .unwrap()
            .clone()
            .unwrap()
            .as_i64()
            .unwrap();
        assert_eq!(duration, 0);

        // Stays enabled even at 0
        let enabled = updates[0]
            .state
            .get("enabled")
            .unwrap()
            .clone()
            .unwrap()
            .as_bool()
            .unwrap();
        assert!(enabled);
    }

    #[test]
    fn test_disabled_countdown_not_updated() {
        let updater = CountdownUpdater;
        let last_tick = Utc::now() - Duration::seconds(5);
        let widget = create_test_widget("test-3", false, 60, last_tick);

        let updates = updater.compute_batch_updates(&[widget]);

        assert_eq!(updates.len(), 0);
    }

    #[test]
    fn test_no_update_if_less_than_one_second() {
        let updater = CountdownUpdater;
        let last_tick = Utc::now() - Duration::milliseconds(500);
        let widget = create_test_widget("test-4", true, 60, last_tick);

        let updates = updater.compute_batch_updates(&[widget]);

        assert_eq!(updates.len(), 0);
    }
}
