// src/model.rs
//
// Database row structs — every field maps 1-to-1 to a column.
// SQLx derives FromRow so we can use query_as!() directly.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ── Workflow ──────────────────────────────────────────────────────────────────

#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct WorkflowModel {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── Node ─────────────────────────────────────────────────────────────────────

#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct NodeModel {
    pub id: Uuid,
    pub workflow_id: Uuid,
    /// Canvas-local id, e.g. "node_1" — used to rebuild the React state.
    pub canvas_id: String,
    pub node_type: String,
    pub label: String,
    pub pos_x: f64,
    pub pos_y: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── Edge ─────────────────────────────────────────────────────────────────────

#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct EdgeModel {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub from_node_canvas_id: String,
    pub to_node_canvas_id: String,
    pub label: String,
    pub created_at: DateTime<Utc>,
}
