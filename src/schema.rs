// src/schema.rs
//
// Request bodies (what the React client sends)  →  CreateXxx / UpdateXxx
// Response bodies (what we return)              →  use model structs directly,
//                                                  or composed response types.

use serde::{Deserialize, Serialize};

// ── Workflow ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateWorkflowBody {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkflowBody {
    pub name: Option<String>,
    pub description: Option<String>,
}

// ── Node ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NodeBody {
    pub canvas_id: String,
    pub node_type: String,
    pub label: String,
    pub pos_x: f64,
    pub pos_y: f64,
}

// ── Edge ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EdgeBody {
    pub from_node_canvas_id: String,
    pub to_node_canvas_id: String,
    #[serde(default)]
    pub label: String,
}

// ── Save diagram (create-or-replace) ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SaveDiagramBody {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub nodes: Vec<NodeBody>,
    pub edges: Vec<EdgeBody>,
}

// ── Workflow response wrappers ────────────────────────────────────────────────

use crate::model::{EdgeModel, NodeModel, ProcessOrderExecutionModel, ProcessOrderModel, WorkflowModel};

#[derive(Debug, Serialize)]
pub struct WorkflowListItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&WorkflowModel> for WorkflowListItem {
    fn from(w: &WorkflowModel) -> Self {
        WorkflowListItem {
            id: w.id.to_string(),
            name: w.name.clone(),
            description: w.description.clone(),
            created_at: w.created_at.to_rfc3339(),
            updated_at: w.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct WorkflowFullResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub nodes: Vec<NodeResponse>,
    pub edges: Vec<EdgeResponse>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct NodeResponse {
    pub id: String,
    pub canvas_id: String,
    pub node_type: String,
    pub label: String,
    pub pos_x: f64,
    pub pos_y: f64,
}

impl From<&NodeModel> for NodeResponse {
    fn from(n: &NodeModel) -> Self {
        NodeResponse {
            id: n.id.to_string(),
            canvas_id: n.canvas_id.clone(),
            node_type: n.node_type.clone(),
            label: n.label.clone(),
            pos_x: n.pos_x,
            pos_y: n.pos_y,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct EdgeResponse {
    pub id: String,
    pub from_node_canvas_id: String,
    pub to_node_canvas_id: String,
    pub label: String,
}

impl From<&EdgeModel> for EdgeResponse {
    fn from(e: &EdgeModel) -> Self {
        EdgeResponse {
            id: e.id.to_string(),
            from_node_canvas_id: e.from_node_canvas_id.clone(),
            to_node_canvas_id: e.to_node_canvas_id.clone(),
            label: e.label.clone(),
        }
    }
}

/// Generic filter for list endpoints (?page=1&limit=10)
#[derive(Debug, Deserialize)]
pub struct FilterOptions {
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

// ── ProcessOrder request bodies ───────────────────────────────────────────────

/// POST /api/orders  — supervisor creates a new process order
#[derive(Debug, Deserialize)]
pub struct CreateProcessOrderBody {
    pub order_number: String,
    #[serde(default)]
    pub description: String,
    /// UUID of the MBR to assign.  Required at creation.
    pub workflow_id: String,
    #[serde(default)]
    pub assigned_to: String,
    #[serde(default)]
    pub scheduled_date: String,
}

/// PATCH /api/orders/:id  — supervisor edits metadata (not execution state)
#[derive(Debug, Deserialize)]
pub struct UpdateProcessOrderBody {
    pub order_number: Option<String>,
    pub description: Option<String>,
    pub workflow_id: Option<String>,
    pub assigned_to: Option<String>,
    pub scheduled_date: Option<String>,
    pub status: Option<String>,
}

/// POST /api/orders/:id/start  — operator opens the order for execution
/// (transitions pending → in_progress, records total_steps)
#[derive(Debug, Deserialize)]
pub struct StartProcessOrderBody {
    pub total_steps: i32,
}

/// POST /api/orders/:id/steps  — operator confirms one step
#[derive(Debug, Deserialize)]
pub struct ConfirmStepBody {
    pub node_canvas_id: String,
    pub node_type: String,
    pub node_label: String,
    pub step_number: i32,
    #[serde(default)]
    pub confirmed_by: String,
    #[serde(default)]
    pub notes: String,
}

// ── ProcessOrder response bodies ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ProcessOrderResponse {
    pub id: String,
    pub order_number: String,
    pub description: String,
    pub workflow_id: Option<String>,
    pub workflow_name: String,
    pub status: String,
    pub assigned_to: String,
    pub scheduled_date: String,
    pub current_step: i32,
    pub total_steps: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&ProcessOrderModel> for ProcessOrderResponse {
    fn from(p: &ProcessOrderModel) -> Self {
        ProcessOrderResponse {
            id: p.id.to_string(),
            order_number: p.order_number.clone(),
            description: p.description.clone(),
            workflow_id: p.workflow_id.map(|u| u.to_string()),
            workflow_name: p.workflow_name.clone(),
            status: p.status.clone(),
            assigned_to: p.assigned_to.clone(),
            scheduled_date: p.scheduled_date.clone(),
            current_step: p.current_step,
            total_steps: p.total_steps,
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProcessOrderExecutionResponse {
    pub id: String,
    pub process_order_id: String,
    pub node_canvas_id: String,
    pub node_type: String,
    pub node_label: String,
    pub step_number: i32,
    pub confirmed_by: String,
    pub notes: String,
    pub confirmed_at: String,
}

impl From<&ProcessOrderExecutionModel> for ProcessOrderExecutionResponse {
    fn from(e: &ProcessOrderExecutionModel) -> Self {
        ProcessOrderExecutionResponse {
            id: e.id.to_string(),
            process_order_id: e.process_order_id.to_string(),
            node_canvas_id: e.node_canvas_id.clone(),
            node_type: e.node_type.clone(),
            node_label: e.node_label.clone(),
            step_number: e.step_number,
            confirmed_by: e.confirmed_by.clone(),
            notes: e.notes.clone(),
            confirmed_at: e.confirmed_at.to_rfc3339(),
        }
    }
}

/// Full process order detail including execution log
#[derive(Debug, Serialize)]
pub struct ProcessOrderDetailResponse {
    pub order: ProcessOrderResponse,
    pub executions: Vec<ProcessOrderExecutionResponse>,
}
