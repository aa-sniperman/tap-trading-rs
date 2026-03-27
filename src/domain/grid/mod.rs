use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::common::GridCellId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridCell {
    pub grid_cell_id: GridCellId,
    pub column_start_at: DateTime<Utc>,
    pub column_end_at: DateTime<Utc>,
    pub price_low: f64,
    pub price_high: f64,
    pub reward_rate_bps: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementRule {
    pub block_past_columns: bool,
    pub block_current_column: bool,
    pub block_one_column_ahead: bool,
}

#[derive(Clone)]
pub struct GridService {
    rules: PlacementRule,
}

impl GridService {
    pub fn new() -> Self {
        Self {
            rules: PlacementRule {
                block_past_columns: true,
                block_current_column: true,
                block_one_column_ahead: true,
            },
        }
    }

    pub fn placement_rules(&self) -> &PlacementRule {
        &self.rules
    }
}
