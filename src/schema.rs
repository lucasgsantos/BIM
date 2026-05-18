// src/schema.rs
//
// Request bodies (what the React client sends)  →  CreateXxx / UpdateXxx
// Response bodies (what we return)              →  use model structs directly,
//                                                  or the combined WorkflowFullResponse

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
    /// canvas-local id ("node_1", "node_2", …)
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
//
// The client sends the *complete* current canvas state in one shot.
// The handler will upsert the workflow row, then DELETE + INSERT all
// nodes and edges (simpler and safer than diffing).

#[derive(Debug, Deserialize)]
pub struct SaveDiagramBody {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub nodes: Vec<NodeBody>,
    pub edges: Vec<EdgeBody>,
}

// ── Response wrappers ─────────────────────────────────────────────────────────

use crate::model::{EdgeModel, NodeModel, WorkflowModel};

/// Thin listing item — used by GET /api/workflows
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

/// Full diagram — used by GET /api/workflows/:id
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
