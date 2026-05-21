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

// ── ProcessOrder ──────────────────────────────────────────────────────────────

#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct ProcessOrderModel {
    pub id: Uuid,
    /// Human-readable order number, e.g. "PO-2024-001". Unique.
    pub order_number: String,
    pub description: String,
    /// FK to workflows — nullable (SET NULL on workflow delete).
    pub workflow_id: Option<Uuid>,
    /// Denormalised snapshot of the MBR name at creation time.
    pub workflow_name: String,
    /// Lifecycle: pending | in_progress | completed | cancelled
    pub status: String,
    pub assigned_to: String,
    pub scheduled_date: String,
    /// 0-based index of the next step to execute.
    pub current_step: i32,
    pub total_steps: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── ProcessOrderExecution ─────────────────────────────────────────────────────

/// Append-only audit log row — one per confirmed node step.
#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct ProcessOrderExecutionModel {
    pub id: Uuid,
    pub process_order_id: Uuid,
    pub node_canvas_id: String,
    pub node_type: String,
    pub node_label: String,
    pub step_number: i32,
    pub confirmed_by: String,
    pub notes: String,
    pub confirmed_at: DateTime<Utc>,
}
