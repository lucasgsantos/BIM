// src/model.rs
//
// Database row structs — every field maps 1-to-1 to a column.
// SQLx derives FromRow so we can use query_as!() directly.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::JsonValue;
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
    pub order_number: String,
    pub description: String,
    pub workflow_id: Option<Uuid>,
    pub workflow_name: String,
    pub status: String,
    pub assigned_to: String,
    pub scheduled_date: String,
    pub current_step: i32,
    pub total_steps: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── ProcessOrderExecution ─────────────────────────────────────────────────────

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

// ═══════════════════════════════════════════════════════════════════════════════
// MASTER DATA MODELS
// ═══════════════════════════════════════════════════════════════════════════════

// ── Material ──────────────────────────────────────────────────────────────────

#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct MaterialModel {
    pub id: Uuid,
    /// SAP-style material number, e.g. "MAT-10001". Unique.
    pub material_number: String,
    pub name: String,
    pub description: String,
    /// raw | intermediate | finished | consumable | packaging
    pub material_type: String,
    /// Base unit of measure: kg, L, g, mL, units, m, m2, m3
    pub unit_of_measure: String,
    /// Flexible JSONB attributes (viscosity, grade, hazard class, …)
    pub attributes: JsonValue,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── Location ──────────────────────────────────────────────────────────────────

#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct LocationModel {
    pub id: Uuid,
    /// Short code, e.g. "WH-A1-R3". Unique.
    pub location_code: String,
    pub name: String,
    pub description: String,
    /// warehouse | production | lab | quarantine | staging | dispatch
    pub location_type: String,
    /// Self-referential parent for hierarchy (nullable).
    pub parent_location_id: Option<Uuid>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── Batch ─────────────────────────────────────────────────────────────────────

#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct BatchModel {
    pub id: Uuid,
    /// Batch / lot number, e.g. "BAT-2024-00042". Unique.
    pub batch_number: String,
    pub material_id: Uuid,
    /// Denormalised snapshots — survive material renames.
    pub material_number: String,
    pub material_name: String,
    pub quantity: sqlx::types::BigDecimal,
    pub unit_of_measure: String,
    /// active | quarantine | released | expired | consumed | rejected
    pub status: String,
    pub manufactured_date: String,
    pub expiry_date: String,
    pub notes: String,
    pub process_order_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── HandlingUnit ──────────────────────────────────────────────────────────────

#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct HandlingUnitModel {
    pub id: Uuid,
    /// Unique HU identifier / barcode, e.g. "HU-00000123".
    pub hu_number: String,
    pub description: String,
    /// pallet | container | box | drum | ibc | sack | other
    pub hu_type: String,
    /// empty | partial | full | sealed | damaged | disposed
    pub status: String,
    pub batch_id: Option<Uuid>,
    /// Denormalised snapshot.
    pub batch_number: String,
    pub quantity: sqlx::types::BigDecimal,
    pub unit_of_measure: String,
    pub location_id: Option<Uuid>,
    /// Denormalised snapshot.
    pub location_code: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
