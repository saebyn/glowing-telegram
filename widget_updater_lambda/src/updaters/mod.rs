pub mod countdown;

use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Represents a state update for a widget
#[derive(Debug, Clone)]
pub struct WidgetUpdate {
    pub id: String,
    pub state: HashMap<String, Option<JsonValue>>,
}

/// Trait for widget type-specific update logic
pub trait WidgetUpdater: Send + Sync {
    /// Compute state updates for a batch of widgets
    fn compute_batch_updates(
        &self,
        widgets: &[crate::StreamWidget],
    ) -> Vec<WidgetUpdate>;
}
