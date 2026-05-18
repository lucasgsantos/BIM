// src/handler.rs
//
// Route handlers — mirror the pattern from wpcodevo/rust-postgres-crud-sqlx:
//   list     GET  /api/workflows
//   create   POST /api/workflows
//   get one  GET  /api/workflows/{id}
//   save     PUT  /api/workflows/{id}/diagram   ← replaces nodes+edges atomically
//   delete   DELETE /api/workflows/{id}

use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::model::{EdgeModel, NodeModel, WorkflowModel};
use crate::schema::{
    CreateWorkflowBody, EdgeResponse, FilterOptions, NodeResponse, SaveDiagramBody,
    WorkflowFullResponse, WorkflowListItem,
};

// ── GET /api/healthchecker ────────────────────────────────────────────────────

#[get("/api/healthchecker")]
pub async fn health_checker() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "status": "success",
        "message": "Workflow Designer API — Rust + SQLX + PostgreSQL"
    }))
}

// ── GET /api/workflows ────────────────────────────────────────────────────────
//
// Returns a list of workflow metadata (no nodes/edges for performance).
// Supports ?page=1&limit=10

#[get("/api/workflows")]
pub async fn list_workflows(
    opts: web::Query<FilterOptions>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let limit = opts.limit.unwrap_or(20) as i64;
    let offset = ((opts.page.unwrap_or(1) - 1) * opts.limit.unwrap_or(20)) as i64;

    let rows = sqlx::query_as!(
        WorkflowModel,
        r#"SELECT id, name, description, created_at, updated_at
           FROM workflows
           ORDER BY updated_at DESC
           LIMIT $1 OFFSET $2"#,
        limit,
        offset
    )
    .fetch_all(db.get_ref())
    .await;

    match rows {
        Ok(workflows) => {
            let items: Vec<WorkflowListItem> = workflows.iter().map(|w| w.into()).collect();
            HttpResponse::Ok().json(json!({
                "status": "success",
                "results": items.len(),
                "workflows": items
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "status": "error",
            "message": format!("{:?}", e)
        })),
    }
}

// ── POST /api/workflows ───────────────────────────────────────────────────────
//
// Creates an empty workflow record (no nodes/edges yet).
// Use PUT /{id}/diagram to persist the full canvas.

#[post("/api/workflows")]
pub async fn create_workflow(
    body: web::Json<CreateWorkflowBody>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let id = Uuid::new_v4();
    let now = Utc::now();

    let row = sqlx::query_as!(
        WorkflowModel,
        r#"INSERT INTO workflows (id, name, description, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id, name, description, created_at, updated_at"#,
        id,
        body.name.trim(),
        body.description.trim(),
        now,
        now
    )
    .fetch_one(db.get_ref())
    .await;

    match row {
        Ok(w) => HttpResponse::Created().json(json!({
            "status": "success",
            "workflow": WorkflowListItem::from(&w)
        })),
        Err(e) => {
            if e.to_string().contains("duplicate") {
                HttpResponse::Conflict().json(json!({
                    "status": "error",
                    "message": "A workflow with that name already exists"
                }))
            } else {
                HttpResponse::InternalServerError().json(json!({
                    "status": "error",
                    "message": format!("{:?}", e)
                }))
            }
        }
    }
}

// ── GET /api/workflows/{id} ───────────────────────────────────────────────────
//
// Returns the full diagram: workflow metadata + all nodes + all edges.

#[get("/api/workflows/{id}")]
pub async fn get_workflow(
    path: web::Path<String>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({
                "status": "error",
                "message": "Invalid UUID"
            }))
        }
    };

    // Fetch workflow row
    let wf = sqlx::query_as!(
        WorkflowModel,
        r#"SELECT id, name, description, created_at, updated_at
           FROM workflows WHERE id = $1"#,
        id
    )
    .fetch_optional(db.get_ref())
    .await;

    let workflow = match wf {
        Ok(Some(w)) => w,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({
                "status": "error",
                "message": "Workflow not found"
            }))
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "status": "error",
                "message": format!("{:?}", e)
            }))
        }
    };

    // Fetch nodes
    let nodes = sqlx::query_as!(
        NodeModel,
        r#"SELECT id, workflow_id, canvas_id, node_type, label, pos_x, pos_y, created_at, updated_at
           FROM wf_nodes WHERE workflow_id = $1 ORDER BY created_at ASC"#,
        id
    )
    .fetch_all(db.get_ref())
    .await;

    let node_list = match nodes {
        Ok(ns) => ns,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "status": "error",
                "message": format!("{:?}", e)
            }))
        }
    };

    // Fetch edges
    let edges = sqlx::query_as!(
        EdgeModel,
        r#"SELECT id, workflow_id, from_node_canvas_id, to_node_canvas_id, label, created_at
           FROM wf_edges WHERE workflow_id = $1 ORDER BY created_at ASC"#,
        id
    )
    .fetch_all(db.get_ref())
    .await;

    let edge_list = match edges {
        Ok(es) => es,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "status": "error",
                "message": format!("{:?}", e)
            }))
        }
    };

    let response = WorkflowFullResponse {
        id: workflow.id.to_string(),
        name: workflow.name.clone(),
        description: workflow.description.clone(),
        nodes: node_list.iter().map(NodeResponse::from).collect(),
        edges: edge_list.iter().map(EdgeResponse::from).collect(),
        created_at: workflow.created_at.to_rfc3339(),
        updated_at: workflow.updated_at.to_rfc3339(),
    };

    HttpResponse::Ok().json(json!({
        "status": "success",
        "workflow": response
    }))
}

// ── PUT /api/workflows/{id}/diagram ──────────────────────────────────────────
//
// SAVE the full diagram.
// Strategy: wrap everything in a transaction, delete all existing
// nodes + edges for this workflow, then bulk-insert the new state.
// This is simpler and safer than diffing — the canvas sends the
// complete truth every time.

#[put("/api/workflows/{id}/diagram")]
pub async fn save_diagram(
    path: web::Path<String>,
    body: web::Json<SaveDiagramBody>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({
                "status": "error",
                "message": "Invalid UUID"
            }))
        }
    };

    let mut tx = match db.begin().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "status": "error",
                "message": format!("Could not start transaction: {:?}", e)
            }))
        }
    };

    // 1. Update workflow metadata
    let now = Utc::now();
    let update = sqlx::query!(
        r#"UPDATE workflows
           SET name = $1, description = $2, updated_at = $3
           WHERE id = $4"#,
        body.name.trim(),
        body.description.trim(),
        now,
        id
    )
    .execute(&mut tx)
    .await;

    if let Err(e) = update {
        let _ = tx.rollback().await;
        return HttpResponse::InternalServerError().json(json!({
            "status": "error",
            "message": format!("{:?}", e)
        }));
    }

    if update.unwrap().rows_affected() == 0 {
        let _ = tx.rollback().await;
        return HttpResponse::NotFound().json(json!({
            "status": "error",
            "message": "Workflow not found"
        }));
    }

    // 2. Delete existing nodes (edges cascade via FK)
    if let Err(e) = sqlx::query!("DELETE FROM wf_nodes WHERE workflow_id = $1", id)
        .execute(&mut tx)
        .await
    {
        let _ = tx.rollback().await;
        return HttpResponse::InternalServerError().json(json!({
            "status": "error",
            "message": format!("{:?}", e)
        }));
    }

    // 3. Delete existing edges (belt-and-suspenders after cascade)
    if let Err(e) = sqlx::query!("DELETE FROM wf_edges WHERE workflow_id = $1", id)
        .execute(&mut tx)
        .await
    {
        let _ = tx.rollback().await;
        return HttpResponse::InternalServerError().json(json!({
            "status": "error",
            "message": format!("{:?}", e)
        }));
    }

    // 4. Insert new nodes
    for node in &body.nodes {
        let node_id = Uuid::new_v4();
        let node_now = Utc::now();
        if let Err(e) = sqlx::query!(
            r#"INSERT INTO wf_nodes
               (id, workflow_id, canvas_id, node_type, label, pos_x, pos_y, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
            node_id,
            id,
            node.canvas_id,
            node.node_type,
            node.label,
            node.pos_x,
            node.pos_y,
            node_now,
            node_now
        )
        .execute(&mut tx)
        .await
        {
            let _ = tx.rollback().await;
            return HttpResponse::InternalServerError().json(json!({
                "status": "error",
                "message": format!("Node insert failed: {:?}", e)
            }));
        }
    }

    // 5. Insert new edges
    for edge in &body.edges {
        let edge_id = Uuid::new_v4();
        let edge_now = Utc::now();
        if let Err(e) = sqlx::query!(
            r#"INSERT INTO wf_edges
               (id, workflow_id, from_node_canvas_id, to_node_canvas_id, label, created_at)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
            edge_id,
            id,
            edge.from_node_canvas_id,
            edge.to_node_canvas_id,
            edge.label,
            edge_now
        )
        .execute(&mut tx)
        .await
        {
            let _ = tx.rollback().await;
            return HttpResponse::InternalServerError().json(json!({
                "status": "error",
                "message": format!("Edge insert failed: {:?}", e)
            }));
        }
    }

    // 6. Commit
    if let Err(e) = tx.commit().await {
        return HttpResponse::InternalServerError().json(json!({
            "status": "error",
            "message": format!("Commit failed: {:?}", e)
        }));
    }

    HttpResponse::Ok().json(json!({
        "status": "success",
        "message": "Diagram saved successfully",
        "node_count": body.nodes.len(),
        "edge_count": body.edges.len()
    }))
}

// ── DELETE /api/workflows/{id} ────────────────────────────────────────────────

#[delete("/api/workflows/{id}")]
pub async fn delete_workflow(
    path: web::Path<String>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({
                "status": "error",
                "message": "Invalid UUID"
            }))
        }
    };

    let result = sqlx::query!("DELETE FROM workflows WHERE id = $1", id)
        .execute(db.get_ref())
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::NoContent().finish(),
        Ok(_) => HttpResponse::NotFound().json(json!({
            "status": "error",
            "message": "Workflow not found"
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "status": "error",
            "message": format!("{:?}", e)
        })),
    }
}
