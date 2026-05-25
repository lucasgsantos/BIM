// src/schema.rs
//
// Request bodies  →  CreateXxx / UpdateXxx / PatchXxx
// Response bodies →  XxxResponse, XxxListItem
//
// Convention:
//   * All IDs in responses are String (UUID serialised as text).
//   * Numeric fields (quantity) are serialised as f64 for JSON compatibility.
//   * PATCH bodies use Option<T> so the client only sends changed fields.

use serde::{Deserialize, Serialize};

use crate::model::{
    BatchModel, EdgeModel, HandlingUnitModel, LocationModel, MaterialModel,
    NodeModel, ProcessOrderExecutionModel, ProcessOrderModel, WorkflowModel,
};

// ─────────────────────────────────────────────────────────────────────────────
// SHARED HELPERS
// ─────────────────────────────────────────────────────────────────────────────

/// Generic filter for list endpoints (?page=1&limit=50)
#[derive(Debug, Deserialize)]
pub struct FilterOptions {
    pub page:  Option<usize>,
    pub limit: Option<usize>,
}

// ─────────────────────────────────────────────────────────────────────────────
// WORKFLOW (MBR)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateWorkflowBody {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkflowBody {
    pub name:        Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NodeBody {
    pub canvas_id: String,
    pub node_type: String,
    pub label:     String,
    pub pos_x:     f64,
    pub pos_y:     f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EdgeBody {
    pub from_node_canvas_id: String,
    pub to_node_canvas_id:   String,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveDiagramBody {
    pub name:        String,
    #[serde(default)]
    pub description: String,
    pub nodes:       Vec<NodeBody>,
    pub edges:       Vec<EdgeBody>,
}

// ── Workflow responses ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct WorkflowListItem {
    pub id:          String,
    pub name:        String,
    pub description: String,
    pub created_at:  String,
    pub updated_at:  String,
}
impl From<&WorkflowModel> for WorkflowListItem {
    fn from(w: &WorkflowModel) -> Self {
        WorkflowListItem {
            id:          w.id.to_string(),
            name:        w.name.clone(),
            description: w.description.clone(),
            created_at:  w.created_at.to_rfc3339(),
            updated_at:  w.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct WorkflowFullResponse {
    pub id:          String,
    pub name:        String,
    pub description: String,
    pub nodes:       Vec<NodeResponse>,
    pub edges:       Vec<EdgeResponse>,
    pub created_at:  String,
    pub updated_at:  String,
}

#[derive(Debug, Serialize)]
pub struct NodeResponse {
    pub id:        String,
    pub canvas_id: String,
    pub node_type: String,
    pub label:     String,
    pub pos_x:     f64,
    pub pos_y:     f64,
}
impl From<&NodeModel> for NodeResponse {
    fn from(n: &NodeModel) -> Self {
        NodeResponse {
            id:        n.id.to_string(),
            canvas_id: n.canvas_id.clone(),
            node_type: n.node_type.clone(),
            label:     n.label.clone(),
            pos_x:     n.pos_x,
            pos_y:     n.pos_y,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct EdgeResponse {
    pub id:                  String,
    pub from_node_canvas_id: String,
    pub to_node_canvas_id:   String,
    pub label:               String,
}
impl From<&EdgeModel> for EdgeResponse {
    fn from(e: &EdgeModel) -> Self {
        EdgeResponse {
            id:                  e.id.to_string(),
            from_node_canvas_id: e.from_node_canvas_id.clone(),
            to_node_canvas_id:   e.to_node_canvas_id.clone(),
            label:               e.label.clone(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PROCESS ORDER
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateProcessOrderBody {
    pub order_number: String,
    #[serde(default)]
    pub description: String,
    pub workflow_id:  String,
    #[serde(default)]
    pub assigned_to: String,
    #[serde(default)]
    pub scheduled_date: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProcessOrderBody {
    pub order_number:   Option<String>,
    pub description:    Option<String>,
    pub workflow_id:    Option<String>,
    pub assigned_to:    Option<String>,
    pub scheduled_date: Option<String>,
    pub status:         Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StartProcessOrderBody {
    pub total_steps: i32,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmStepBody {
    pub node_canvas_id: String,
    pub node_type:      String,
    pub node_label:     String,
    pub step_number:    i32,
    #[serde(default)]
    pub confirmed_by: String,
    #[serde(default)]
    pub notes: String,
}

// ── Process Order responses ───────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ProcessOrderResponse {
    pub id:             String,
    pub order_number:   String,
    pub description:    String,
    pub workflow_id:    Option<String>,
    pub workflow_name:  String,
    pub status:         String,
    pub assigned_to:    String,
    pub scheduled_date: String,
    pub current_step:   i32,
    pub total_steps:    i32,
    pub created_at:     String,
    pub updated_at:     String,
}
impl From<&ProcessOrderModel> for ProcessOrderResponse {
    fn from(p: &ProcessOrderModel) -> Self {
        ProcessOrderResponse {
            id:             p.id.to_string(),
            order_number:   p.order_number.clone(),
            description:    p.description.clone(),
            workflow_id:    p.workflow_id.map(|u| u.to_string()),
            workflow_name:  p.workflow_name.clone(),
            status:         p.status.clone(),
            assigned_to:    p.assigned_to.clone(),
            scheduled_date: p.scheduled_date.clone(),
            current_step:   p.current_step,
            total_steps:    p.total_steps,
            created_at:     p.created_at.to_rfc3339(),
            updated_at:     p.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProcessOrderExecutionResponse {
    pub id:               String,
    pub process_order_id: String,
    pub node_canvas_id:   String,
    pub node_type:        String,
    pub node_label:       String,
    pub step_number:      i32,
    pub confirmed_by:     String,
    pub notes:            String,
    pub confirmed_at:     String,
}
impl From<&ProcessOrderExecutionModel> for ProcessOrderExecutionResponse {
    fn from(e: &ProcessOrderExecutionModel) -> Self {
        ProcessOrderExecutionResponse {
            id:               e.id.to_string(),
            process_order_id: e.process_order_id.to_string(),
            node_canvas_id:   e.node_canvas_id.clone(),
            node_type:        e.node_type.clone(),
            node_label:       e.node_label.clone(),
            step_number:      e.step_number,
            confirmed_by:     e.confirmed_by.clone(),
            notes:            e.notes.clone(),
            confirmed_at:     e.confirmed_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProcessOrderDetailResponse {
    pub order:      ProcessOrderResponse,
    pub executions: Vec<ProcessOrderExecutionResponse>,
}

// ─────────────────────────────────────────────────────────────────────────────
// MATERIAL
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateMaterialBody {
    pub material_number: String,
    pub name:            String,
    #[serde(default)]
    pub description:     String,
    /// raw | intermediate | finished | consumable | packaging
    #[serde(default = "default_material_type")]
    pub material_type:   String,
    /// kg | L | g | mL | units | m | m2 | m3
    #[serde(default = "default_uom")]
    pub unit_of_measure: String,
    /// Arbitrary key-value attributes stored as JSON.
    #[serde(default = "default_json_obj")]
    pub attributes:      serde_json::Value,
    #[serde(default = "default_true")]
    pub is_active:       bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMaterialBody {
    pub material_number: Option<String>,
    pub name:            Option<String>,
    pub description:     Option<String>,
    pub material_type:   Option<String>,
    pub unit_of_measure: Option<String>,
    pub attributes:      Option<serde_json::Value>,
    pub is_active:       Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct MaterialResponse {
    pub id:              String,
    pub material_number: String,
    pub name:            String,
    pub description:     String,
    pub material_type:   String,
    pub unit_of_measure: String,
    pub attributes:      serde_json::Value,
    pub is_active:       bool,
    pub created_at:      String,
    pub updated_at:      String,
}
impl From<&MaterialModel> for MaterialResponse {
    fn from(m: &MaterialModel) -> Self {
        MaterialResponse {
            id:              m.id.to_string(),
            material_number: m.material_number.clone(),
            name:            m.name.clone(),
            description:     m.description.clone(),
            material_type:   m.material_type.clone(),
            unit_of_measure: m.unit_of_measure.clone(),
            attributes:      m.attributes.clone(),
            is_active:       m.is_active,
            created_at:      m.created_at.to_rfc3339(),
            updated_at:      m.updated_at.to_rfc3339(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// LOCATION
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateLocationBody {
    pub location_code:     String,
    pub name:              String,
    #[serde(default)]
    pub description:       String,
    /// warehouse | production | lab | quarantine | staging | dispatch
    #[serde(default = "default_location_type")]
    pub location_type:     String,
    pub parent_location_id: Option<String>,
    #[serde(default = "default_true")]
    pub is_active:         bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLocationBody {
    pub location_code:     Option<String>,
    pub name:              Option<String>,
    pub description:       Option<String>,
    pub location_type:     Option<String>,
    pub parent_location_id: Option<String>,   // send empty string "" to clear
    pub is_active:         Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct LocationResponse {
    pub id:                String,
    pub location_code:     String,
    pub name:              String,
    pub description:       String,
    pub location_type:     String,
    pub parent_location_id: Option<String>,
    pub is_active:         bool,
    pub created_at:        String,
    pub updated_at:        String,
}
impl From<&LocationModel> for LocationResponse {
    fn from(l: &LocationModel) -> Self {
        LocationResponse {
            id:                l.id.to_string(),
            location_code:     l.location_code.clone(),
            name:              l.name.clone(),
            description:       l.description.clone(),
            location_type:     l.location_type.clone(),
            parent_location_id: l.parent_location_id.map(|u| u.to_string()),
            is_active:         l.is_active,
            created_at:        l.created_at.to_rfc3339(),
            updated_at:        l.updated_at.to_rfc3339(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// BATCH
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateBatchBody {
    pub batch_number:      String,
    pub material_id:       String,
    pub quantity:          f64,
    #[serde(default = "default_uom")]
    pub unit_of_measure:   String,
    /// active | quarantine | released | expired | consumed | rejected
    #[serde(default = "default_batch_status")]
    pub status:            String,
    #[serde(default)]
    pub manufactured_date: String,
    #[serde(default)]
    pub expiry_date:       String,
    #[serde(default)]
    pub notes:             String,
    pub process_order_id:  Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBatchBody {
    pub batch_number:      Option<String>,
    pub material_id:       Option<String>,
    pub quantity:          Option<f64>,
    pub unit_of_measure:   Option<String>,
    pub status:            Option<String>,
    pub manufactured_date: Option<String>,
    pub expiry_date:       Option<String>,
    pub notes:             Option<String>,
    pub process_order_id:  Option<String>,  // "" clears
}

#[derive(Debug, Serialize)]
pub struct BatchResponse {
    pub id:                String,
    pub batch_number:      String,
    pub material_id:       String,
    pub material_number:   String,
    pub material_name:     String,
    pub quantity:          f64,
    pub unit_of_measure:   String,
    pub status:            String,
    pub manufactured_date: String,
    pub expiry_date:       String,
    pub notes:             String,
    pub process_order_id:  Option<String>,
    pub created_at:        String,
    pub updated_at:        String,
}
impl From<&BatchModel> for BatchResponse {
    fn from(b: &BatchModel) -> Self {
        use std::str::FromStr;
        let qty = f64::from_str(&b.quantity.to_string()).unwrap_or(0.0);
        BatchResponse {
            id:                b.id.to_string(),
            batch_number:      b.batch_number.clone(),
            material_id:       b.material_id.to_string(),
            material_number:   b.material_number.clone(),
            material_name:     b.material_name.clone(),
            quantity:          qty,
            unit_of_measure:   b.unit_of_measure.clone(),
            status:            b.status.clone(),
            manufactured_date: b.manufactured_date.clone(),
            expiry_date:       b.expiry_date.clone(),
            notes:             b.notes.clone(),
            process_order_id:  b.process_order_id.map(|u| u.to_string()),
            created_at:        b.created_at.to_rfc3339(),
            updated_at:        b.updated_at.to_rfc3339(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// HANDLING UNIT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateHandlingUnitBody {
    pub hu_number:       String,
    #[serde(default)]
    pub description:     String,
    /// pallet | container | box | drum | ibc | sack | other
    #[serde(default = "default_hu_type")]
    pub hu_type:         String,
    /// empty | partial | full | sealed | damaged | disposed
    #[serde(default = "default_hu_status")]
    pub status:          String,
    pub batch_id:        Option<String>,
    pub quantity:        Option<f64>,
    #[serde(default = "default_uom")]
    pub unit_of_measure: String,
    pub location_id:     Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateHandlingUnitBody {
    pub hu_number:       Option<String>,
    pub description:     Option<String>,
    pub hu_type:         Option<String>,
    pub status:          Option<String>,
    pub batch_id:        Option<String>,   // "" clears
    pub quantity:        Option<f64>,
    pub unit_of_measure: Option<String>,
    pub location_id:     Option<String>,   // "" clears
}

#[derive(Debug, Serialize)]
pub struct HandlingUnitResponse {
    pub id:              String,
    pub hu_number:       String,
    pub description:     String,
    pub hu_type:         String,
    pub status:          String,
    pub batch_id:        Option<String>,
    pub batch_number:    String,
    pub quantity:        f64,
    pub unit_of_measure: String,
    pub location_id:     Option<String>,
    pub location_code:   String,
    pub created_at:      String,
    pub updated_at:      String,
}
impl From<&HandlingUnitModel> for HandlingUnitResponse {
    fn from(h: &HandlingUnitModel) -> Self {
        use std::str::FromStr;
        let qty = f64::from_str(&h.quantity.to_string()).unwrap_or(0.0);
        HandlingUnitResponse {
            id:              h.id.to_string(),
            hu_number:       h.hu_number.clone(),
            description:     h.description.clone(),
            hu_type:         h.hu_type.clone(),
            status:          h.status.clone(),
            batch_id:        h.batch_id.map(|u| u.to_string()),
            batch_number:    h.batch_number.clone(),
            quantity:        qty,
            unit_of_measure: h.unit_of_measure.clone(),
            location_id:     h.location_id.map(|u| u.to_string()),
            location_code:   h.location_code.clone(),
            created_at:      h.created_at.to_rfc3339(),
            updated_at:      h.updated_at.to_rfc3339(),
        }
    }
}

// ── Default-value helpers (used by serde) ─────────────────────────────────────

fn default_true()           -> bool             { true }
fn default_material_type()  -> String          { "raw".to_string() }
fn default_location_type()  -> String          { "warehouse".to_string() }
fn default_batch_status()   -> String          { "active".to_string() }
fn default_hu_type()        -> String          { "pallet".to_string() }
fn default_hu_status()      -> String          { "empty".to_string() }
fn default_uom()            -> String          { "kg".to_string() }
fn default_json_obj()       -> serde_json::Value { serde_json::json!({}) }
