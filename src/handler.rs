// src/handler.rs
//
// Route handlers for the BIM MES API.
//
// Workflow (MBR) endpoints:
//   GET    /api/healthchecker
//   GET    /api/workflows
//   POST   /api/workflows
//   GET    /api/workflows/:id
//   PUT    /api/workflows/:id/diagram
//   DELETE /api/workflows/:id
//
// Process Order (EBR) endpoints:
//   GET    /api/orders                    list all orders (with filter by status)
//   POST   /api/orders                    supervisor creates an order
//   GET    /api/orders/:id                get order + full execution log
//   PATCH  /api/orders/:id                supervisor edits metadata / status
//   DELETE /api/orders/:id                supervisor deletes an order
//   POST   /api/orders/:id/start          operator starts execution
//   POST   /api/orders/:id/steps          operator confirms a step
//   POST   /api/orders/:id/complete       operator marks order complete
//   POST   /api/orders/:id/cancel         supervisor cancels an order

use actix_web::{delete, get, patch, post, put, web, HttpResponse, Responder};
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::model::{EdgeModel, NodeModel, ProcessOrderExecutionModel, ProcessOrderModel, WorkflowModel};
use crate::schema::{
    ConfirmStepBody, CreateProcessOrderBody, CreateWorkflowBody, EdgeResponse, FilterOptions,
    NodeResponse, ProcessOrderDetailResponse, ProcessOrderExecutionResponse, ProcessOrderResponse,
    SaveDiagramBody, StartProcessOrderBody, UpdateProcessOrderBody, WorkflowFullResponse,
    WorkflowListItem,
};

// ── Health ────────────────────────────────────────────────────────────────────

#[get("/api/healthchecker")]
pub async fn health_checker() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "status": "success",
        "message": "BIM MES API — Rust + SQLx + PostgreSQL"
    }))
}

// ═══════════════════════════════════════════════════════════════════════════════
// WORKFLOW (MBR) HANDLERS
// ═══════════════════════════════════════════════════════════════════════════════

// ── GET /api/workflows ────────────────────────────────────────────────────────

#[get("/api/workflows")]
pub async fn list_workflows(
    opts: web::Query<FilterOptions>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let limit  = opts.limit.unwrap_or(20) as i64;
    let offset = ((opts.page.unwrap_or(1) - 1) * opts.limit.unwrap_or(20)) as i64;

    match sqlx::query_as!(
        WorkflowModel,
        r#"SELECT id, name, description, created_at, updated_at
           FROM workflows
           ORDER BY updated_at DESC
           LIMIT $1 OFFSET $2"#,
        limit, offset
    )
    .fetch_all(db.get_ref())
    .await
    {
        Ok(workflows) => {
            let items: Vec<WorkflowListItem> = workflows.iter().map(|w| w.into()).collect();
            HttpResponse::Ok().json(json!({ "status": "success", "results": items.len(), "workflows": items }))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

// ── POST /api/workflows ───────────────────────────────────────────────────────

#[post("/api/workflows")]
pub async fn create_workflow(
    body: web::Json<CreateWorkflowBody>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let id  = Uuid::new_v4();
    let now = Utc::now();

    match sqlx::query_as!(
        WorkflowModel,
        r#"INSERT INTO workflows (id, name, description, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id, name, description, created_at, updated_at"#,
        id, body.name.trim(), body.description.trim(), now, now
    )
    .fetch_one(db.get_ref())
    .await
    {
        Ok(w) => HttpResponse::Created()
            .json(json!({ "status": "success", "workflow": WorkflowListItem::from(&w) })),
        Err(e) => {
            if e.to_string().contains("duplicate") {
                HttpResponse::Conflict()
                    .json(json!({ "status": "error", "message": "A workflow with that name already exists" }))
            } else {
                HttpResponse::InternalServerError()
                    .json(json!({ "status": "error", "message": format!("{:?}", e) }))
            }
        }
    }
}

// ── GET /api/workflows/:id ────────────────────────────────────────────────────

#[get("/api/workflows/{id}")]
pub async fn get_workflow(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };

    let wf = match sqlx::query_as!(
        WorkflowModel,
        "SELECT id, name, description, created_at, updated_at FROM workflows WHERE id = $1",
        id
    )
    .fetch_optional(db.get_ref())
    .await
    {
        Ok(Some(w)) => w,
        Ok(None)    => return HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Workflow not found" })),
        Err(e)      => return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    };

    let nodes = match sqlx::query_as!(
        NodeModel,
        r#"SELECT id, workflow_id, canvas_id, node_type, label, pos_x, pos_y, created_at, updated_at
           FROM wf_nodes WHERE workflow_id = $1 ORDER BY created_at ASC"#,
        id
    )
    .fetch_all(db.get_ref())
    .await
    {
        Ok(ns) => ns,
        Err(e) => return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    };

    let edges = match sqlx::query_as!(
        EdgeModel,
        r#"SELECT id, workflow_id, from_node_canvas_id, to_node_canvas_id, label, created_at
           FROM wf_edges WHERE workflow_id = $1 ORDER BY created_at ASC"#,
        id
    )
    .fetch_all(db.get_ref())
    .await
    {
        Ok(es) => es,
        Err(e) => return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    };

    HttpResponse::Ok().json(json!({
        "status": "success",
        "workflow": WorkflowFullResponse {
            id: wf.id.to_string(),
            name: wf.name.clone(),
            description: wf.description.clone(),
            nodes: nodes.iter().map(NodeResponse::from).collect(),
            edges: edges.iter().map(EdgeResponse::from).collect(),
            created_at: wf.created_at.to_rfc3339(),
            updated_at: wf.updated_at.to_rfc3339(),
        }
    }))
}

// ── PUT /api/workflows/:id/diagram ────────────────────────────────────────────

#[put("/api/workflows/{id}/diagram")]
pub async fn save_diagram(
    path: web::Path<String>,
    body: web::Json<SaveDiagramBody>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };

    let mut tx = match db.begin().await {
        Ok(t)  => t,
        Err(e) => return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("Transaction error: {:?}", e) })),
    };

    let now = Utc::now();

    let update = sqlx::query!(
        "UPDATE workflows SET name = $1, description = $2, updated_at = $3 WHERE id = $4",
        body.name.trim(), body.description.trim(), now, id
    )
    .execute(&mut *tx)
    .await;

    match update {
        Err(e) => { let _ = tx.rollback().await; return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })); }
        Ok(r) if r.rows_affected() == 0 => { let _ = tx.rollback().await; return HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Workflow not found" })); }
        _ => {}
    }

    if let Err(e) = sqlx::query!("DELETE FROM wf_nodes WHERE workflow_id = $1", id)
        .execute(&mut *tx).await
    { let _ = tx.rollback().await; return HttpResponse::InternalServerError()
        .json(json!({ "status": "error", "message": format!("{:?}", e) })); }

    if let Err(e) = sqlx::query!("DELETE FROM wf_edges WHERE workflow_id = $1", id)
        .execute(&mut *tx).await
    { let _ = tx.rollback().await; return HttpResponse::InternalServerError()
        .json(json!({ "status": "error", "message": format!("{:?}", e) })); }

    for node in &body.nodes {
        let node_id  = Uuid::new_v4();
        let node_now = Utc::now();
        if let Err(e) = sqlx::query!(
            r#"INSERT INTO wf_nodes (id, workflow_id, canvas_id, node_type, label, pos_x, pos_y, created_at, updated_at)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)"#,
            node_id, id, node.canvas_id, node.node_type, node.label,
            node.pos_x, node.pos_y, node_now, node_now
        ).execute(&mut *tx).await
        { let _ = tx.rollback().await; return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("Node insert: {:?}", e) })); }
    }

    for edge in &body.edges {
        let edge_id  = Uuid::new_v4();
        let edge_now = Utc::now();
        if let Err(e) = sqlx::query!(
            r#"INSERT INTO wf_edges (id, workflow_id, from_node_canvas_id, to_node_canvas_id, label, created_at)
               VALUES ($1,$2,$3,$4,$5,$6)"#,
            edge_id, id, edge.from_node_canvas_id, edge.to_node_canvas_id, edge.label, edge_now
        ).execute(&mut *tx).await
        { let _ = tx.rollback().await; return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("Edge insert: {:?}", e) })); }
    }

    if let Err(e) = tx.commit().await {
        return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("Commit failed: {:?}", e) }));
    }

    HttpResponse::Ok().json(json!({
        "status": "success",
        "message": "Diagram saved successfully",
        "node_count": body.nodes.len(),
        "edge_count": body.edges.len()
    }))
}

// ── DELETE /api/workflows/:id ─────────────────────────────────────────────────

#[delete("/api/workflows/{id}")]
pub async fn delete_workflow(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };

    match sqlx::query!("DELETE FROM workflows WHERE id = $1", id)
        .execute(db.get_ref()).await
    {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::NoContent().finish(),
        Ok(_) => HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Workflow not found" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// PROCESS ORDER HANDLERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Query params for GET /api/orders — optional status filter
#[derive(Debug, serde::Deserialize)]
pub struct OrderFilterOptions {
    pub page:   Option<usize>,
    pub limit:  Option<usize>,
    pub status: Option<String>,
}

// ── GET /api/orders ───────────────────────────────────────────────────────────

#[get("/api/orders")]
pub async fn list_orders(
    opts: web::Query<OrderFilterOptions>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let limit  = opts.limit.unwrap_or(50) as i64;
    let offset = ((opts.page.unwrap_or(1) - 1) * opts.limit.unwrap_or(50)) as i64;

    // Optional status filter
    let rows = if let Some(ref status) = opts.status {
        sqlx::query_as!(
            ProcessOrderModel,
            r#"SELECT id, order_number, description, workflow_id, workflow_name,
                      status, assigned_to, scheduled_date, current_step, total_steps,
                      created_at, updated_at
               FROM process_orders
               WHERE status = $1
               ORDER BY updated_at DESC
               LIMIT $2 OFFSET $3"#,
            status, limit, offset
        )
        .fetch_all(db.get_ref())
        .await
    } else {
        sqlx::query_as!(
            ProcessOrderModel,
            r#"SELECT id, order_number, description, workflow_id, workflow_name,
                      status, assigned_to, scheduled_date, current_step, total_steps,
                      created_at, updated_at
               FROM process_orders
               ORDER BY updated_at DESC
               LIMIT $1 OFFSET $2"#,
            limit, offset
        )
        .fetch_all(db.get_ref())
        .await
    };

    match rows {
        Ok(orders) => {
            let items: Vec<ProcessOrderResponse> = orders.iter().map(|o| o.into()).collect();
            HttpResponse::Ok().json(json!({ "status": "success", "results": items.len(), "orders": items }))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

// ── POST /api/orders ──────────────────────────────────────────────────────────

#[post("/api/orders")]
pub async fn create_order(
    body: web::Json<CreateProcessOrderBody>,
    db: web::Data<PgPool>,
) -> impl Responder {
    // Parse and validate workflow_id
    let workflow_uuid = match Uuid::parse_str(&body.workflow_id) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid workflow_id UUID" })),
    };

    // Fetch workflow name snapshot
    let workflow_name = match sqlx::query_scalar!(
        "SELECT name FROM workflows WHERE id = $1",
        workflow_uuid
    )
    .fetch_optional(db.get_ref())
    .await
    {
        Ok(Some(n)) => n,
        Ok(None)    => return HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Workflow not found" })),
        Err(e)      => return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    };

    let id  = Uuid::new_v4();
    let now = Utc::now();

    match sqlx::query_as!(
        ProcessOrderModel,
        r#"INSERT INTO process_orders
               (id, order_number, description, workflow_id, workflow_name,
                status, assigned_to, scheduled_date, current_step, total_steps,
                created_at, updated_at)
           VALUES ($1,$2,$3,$4,$5,'pending',$6,$7,0,0,$8,$9)
           RETURNING id, order_number, description, workflow_id, workflow_name,
                     status, assigned_to, scheduled_date, current_step, total_steps,
                     created_at, updated_at"#,
        id,
        body.order_number.trim(),
        body.description.trim(),
        workflow_uuid,
        workflow_name,
        body.assigned_to.trim(),
        body.scheduled_date.trim(),
        now, now
    )
    .fetch_one(db.get_ref())
    .await
    {
        Ok(order) => HttpResponse::Created()
            .json(json!({ "status": "success", "order": ProcessOrderResponse::from(&order) })),
        Err(e) => {
            if e.to_string().contains("duplicate") {
                HttpResponse::Conflict()
                    .json(json!({ "status": "error", "message": "An order with that number already exists" }))
            } else {
                HttpResponse::InternalServerError()
                    .json(json!({ "status": "error", "message": format!("{:?}", e) }))
            }
        }
    }
}

// ── GET /api/orders/:id ───────────────────────────────────────────────────────

#[get("/api/orders/{id}")]
pub async fn get_order(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };

    let order = match sqlx::query_as!(
        ProcessOrderModel,
        r#"SELECT id, order_number, description, workflow_id, workflow_name,
                  status, assigned_to, scheduled_date, current_step, total_steps,
                  created_at, updated_at
           FROM process_orders WHERE id = $1"#,
        id
    )
    .fetch_optional(db.get_ref())
    .await
    {
        Ok(Some(o)) => o,
        Ok(None)    => return HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Process order not found" })),
        Err(e)      => return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    };

    let executions = match sqlx::query_as!(
        ProcessOrderExecutionModel,
        r#"SELECT id, process_order_id, node_canvas_id, node_type, node_label,
                  step_number, confirmed_by, notes, confirmed_at
           FROM process_order_executions
           WHERE process_order_id = $1
           ORDER BY step_number ASC"#,
        id
    )
    .fetch_all(db.get_ref())
    .await
    {
        Ok(es) => es,
        Err(e) => return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    };

    HttpResponse::Ok().json(json!({
        "status": "success",
        "data": ProcessOrderDetailResponse {
            order: ProcessOrderResponse::from(&order),
            executions: executions.iter().map(ProcessOrderExecutionResponse::from).collect(),
        }
    }))
}

// ── PATCH /api/orders/:id ─────────────────────────────────────────────────────

#[patch("/api/orders/{id}")]
pub async fn update_order(
    path: web::Path<String>,
    body: web::Json<UpdateProcessOrderBody>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };

    // Fetch current row
    let current = match sqlx::query_as!(
        ProcessOrderModel,
        r#"SELECT id, order_number, description, workflow_id, workflow_name,
                  status, assigned_to, scheduled_date, current_step, total_steps,
                  created_at, updated_at
           FROM process_orders WHERE id = $1"#,
        id
    )
    .fetch_optional(db.get_ref())
    .await
    {
        Ok(Some(o)) => o,
        Ok(None)    => return HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Process order not found" })),
        Err(e)      => return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    };

    // Resolve workflow_id / workflow_name if being changed
    let (new_wf_id, new_wf_name) = if let Some(ref wf_id_str) = body.workflow_id {
        let wf_uuid = match Uuid::parse_str(wf_id_str) {
            Ok(u) => u,
            Err(_) => return HttpResponse::BadRequest()
                .json(json!({ "status": "error", "message": "Invalid workflow_id UUID" })),
        };
        match sqlx::query_scalar!("SELECT name FROM workflows WHERE id = $1", wf_uuid)
            .fetch_optional(db.get_ref()).await
        {
            Ok(Some(n)) => (Some(wf_uuid), n),
            Ok(None)    => return HttpResponse::NotFound()
                .json(json!({ "status": "error", "message": "Workflow not found" })),
            Err(e)      => return HttpResponse::InternalServerError()
                .json(json!({ "status": "error", "message": format!("{:?}", e) })),
        }
    } else {
        (current.workflow_id, current.workflow_name.clone())
    };

    let now = Utc::now();
    let new_order_number  = body.order_number.as_deref().unwrap_or(&current.order_number).trim().to_string();
    let new_description   = body.description.as_deref().unwrap_or(&current.description).trim().to_string();
    let new_status        = body.status.as_deref().unwrap_or(&current.status).to_string();
    let new_assigned_to   = body.assigned_to.as_deref().unwrap_or(&current.assigned_to).trim().to_string();
    let new_scheduled     = body.scheduled_date.as_deref().unwrap_or(&current.scheduled_date).trim().to_string();

    match sqlx::query_as!(
        ProcessOrderModel,
        r#"UPDATE process_orders
           SET order_number   = $1,
               description    = $2,
               workflow_id    = $3,
               workflow_name  = $4,
               status         = $5,
               assigned_to    = $6,
               scheduled_date = $7,
               updated_at     = $8
           WHERE id = $9
           RETURNING id, order_number, description, workflow_id, workflow_name,
                     status, assigned_to, scheduled_date, current_step, total_steps,
                     created_at, updated_at"#,
        new_order_number, new_description, new_wf_id, new_wf_name,
        new_status, new_assigned_to, new_scheduled, now, id
    )
    .fetch_one(db.get_ref())
    .await
    {
        Ok(o) => HttpResponse::Ok()
            .json(json!({ "status": "success", "order": ProcessOrderResponse::from(&o) })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

// ── DELETE /api/orders/:id ────────────────────────────────────────────────────

#[delete("/api/orders/{id}")]
pub async fn delete_order(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };

    match sqlx::query!("DELETE FROM process_orders WHERE id = $1", id)
        .execute(db.get_ref()).await
    {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::NoContent().finish(),
        Ok(_)  => HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Process order not found" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

// ── POST /api/orders/:id/start ────────────────────────────────────────────────
/// Transition pending → in_progress.  Records total_steps for progress tracking.

#[post("/api/orders/{id}/start")]
pub async fn start_order(
    path: web::Path<String>,
    body: web::Json<StartProcessOrderBody>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };

    let now = Utc::now();

    match sqlx::query_as!(
        ProcessOrderModel,
        r#"UPDATE process_orders
           SET status      = 'in_progress',
               total_steps = $1,
               current_step = 0,
               updated_at  = $2
           WHERE id = $3 AND status = 'pending'
           RETURNING id, order_number, description, workflow_id, workflow_name,
                     status, assigned_to, scheduled_date, current_step, total_steps,
                     created_at, updated_at"#,
        body.total_steps, now, id
    )
    .fetch_optional(db.get_ref())
    .await
    {
        Ok(Some(o)) => HttpResponse::Ok()
            .json(json!({ "status": "success", "order": ProcessOrderResponse::from(&o) })),
        Ok(None) => HttpResponse::Conflict()
            .json(json!({ "status": "error", "message": "Order not found or not in 'pending' state" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

// ── POST /api/orders/:id/steps ────────────────────────────────────────────────
/// Operator confirms one execution step.  Appends to audit log and advances current_step.

#[post("/api/orders/{id}/steps")]
pub async fn confirm_step(
    path: web::Path<String>,
    body: web::Json<ConfirmStepBody>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };

    // Verify order is in_progress
    let order = match sqlx::query_as!(
        ProcessOrderModel,
        r#"SELECT id, order_number, description, workflow_id, workflow_name,
                  status, assigned_to, scheduled_date, current_step, total_steps,
                  created_at, updated_at
           FROM process_orders WHERE id = $1"#,
        id
    )
    .fetch_optional(db.get_ref())
    .await
    {
        Ok(Some(o)) => o,
        Ok(None)    => return HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Process order not found" })),
        Err(e)      => return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    };

    if order.status != "in_progress" {
        return HttpResponse::Conflict()
            .json(json!({ "status": "error", "message": "Order is not in_progress" }));
    }

    let mut tx = match db.begin().await {
        Ok(t)  => t,
        Err(e) => return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    };

    let exec_id  = Uuid::new_v4();
    let confirmed_at = Utc::now();

    // Insert audit record
    if let Err(e) = sqlx::query!(
        r#"INSERT INTO process_order_executions
               (id, process_order_id, node_canvas_id, node_type, node_label,
                step_number, confirmed_by, notes, confirmed_at)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)"#,
        exec_id, id,
        body.node_canvas_id, body.node_type, body.node_label,
        body.step_number, body.confirmed_by, body.notes, confirmed_at
    )
    .execute(&mut *tx)
    .await
    {
        let _ = tx.rollback().await;
        return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) }));
    }

    // Advance current_step
    let now = Utc::now();
    if let Err(e) = sqlx::query!(
        "UPDATE process_orders SET current_step = $1, updated_at = $2 WHERE id = $3",
        body.step_number, now, id
    )
    .execute(&mut *tx)
    .await
    {
        let _ = tx.rollback().await;
        return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) }));
    }

    if let Err(e) = tx.commit().await {
        return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("Commit: {:?}", e) }));
    }

    HttpResponse::Created().json(json!({
        "status": "success",
        "message": "Step confirmed",
        "step_number": body.step_number
    }))
}

// ── POST /api/orders/:id/complete ─────────────────────────────────────────────
/// Operator signals all steps done → in_progress → completed.

#[post("/api/orders/{id}/complete")]
pub async fn complete_order(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };

    let now = Utc::now();

    match sqlx::query_as!(
        ProcessOrderModel,
        r#"UPDATE process_orders
           SET status = 'completed', updated_at = $1
           WHERE id = $2 AND status = 'in_progress'
           RETURNING id, order_number, description, workflow_id, workflow_name,
                     status, assigned_to, scheduled_date, current_step, total_steps,
                     created_at, updated_at"#,
        now, id
    )
    .fetch_optional(db.get_ref())
    .await
    {
        Ok(Some(o)) => HttpResponse::Ok()
            .json(json!({ "status": "success", "order": ProcessOrderResponse::from(&o) })),
        Ok(None) => HttpResponse::Conflict()
            .json(json!({ "status": "error", "message": "Order not found or not in_progress" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

// ── POST /api/orders/:id/cancel ───────────────────────────────────────────────

#[post("/api/orders/{id}/cancel")]
pub async fn cancel_order(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };

    let now = Utc::now();

    match sqlx::query_as!(
        ProcessOrderModel,
        r#"UPDATE process_orders
           SET status = 'cancelled', updated_at = $1
           WHERE id = $2 AND status IN ('pending','in_progress')
           RETURNING id, order_number, description, workflow_id, workflow_name,
                     status, assigned_to, scheduled_date, current_step, total_steps,
                     created_at, updated_at"#,
        now, id
    )
    .fetch_optional(db.get_ref())
    .await
    {
        Ok(Some(o)) => HttpResponse::Ok()
            .json(json!({ "status": "success", "order": ProcessOrderResponse::from(&o) })),
        Ok(None) => HttpResponse::Conflict()
            .json(json!({ "status": "error", "message": "Order not found or already completed/cancelled" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}
