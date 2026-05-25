// src/handler_master.rs
//
// CRUD handlers for the four master-data entities:
//
//   Material      GET/POST /api/materials      GET/PATCH/DELETE /api/materials/:id
//   Location      GET/POST /api/locations      GET/PATCH/DELETE /api/locations/:id
//   Batch         GET/POST /api/batches        GET/PATCH/DELETE /api/batches/:id
//   HandlingUnit  GET/POST /api/handling-units GET/PATCH/DELETE /api/handling-units/:id
//
// All list endpoints support ?page=&limit= and entity-specific filters.

use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::model::{BatchModel, HandlingUnitModel, LocationModel, MaterialModel};
use crate::schema::{
    BatchResponse, CreateBatchBody, CreateHandlingUnitBody, CreateLocationBody,
    CreateMaterialBody, FilterOptions, HandlingUnitResponse, LocationResponse,
    MaterialResponse, UpdateBatchBody, UpdateHandlingUnitBody, UpdateLocationBody,
    UpdateMaterialBody,
};

// ── Additional list-filter structs ────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct MaterialFilter {
    pub page:          Option<usize>,
    pub limit:         Option<usize>,
    pub material_type: Option<String>,
    pub is_active:     Option<bool>,
    pub search:        Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct LocationFilter {
    pub page:          Option<usize>,
    pub limit:         Option<usize>,
    pub location_type: Option<String>,
    pub is_active:     Option<bool>,
    pub parent_id:     Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct BatchFilter {
    pub page:        Option<usize>,
    pub limit:       Option<usize>,
    pub material_id: Option<String>,
    pub status:      Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct HuFilter {
    pub page:        Option<usize>,
    pub limit:       Option<usize>,
    pub batch_id:    Option<String>,
    pub location_id: Option<String>,
    pub status:      Option<String>,
    pub hu_type:     Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// MATERIAL
// ═══════════════════════════════════════════════════════════════════════════════

// ── GET /api/materials ────────────────────────────────────────────────────────

#[get("/api/materials")]
pub async fn list_materials(
    opts: web::Query<MaterialFilter>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let limit  = opts.limit.unwrap_or(50) as i64;
    let offset = ((opts.page.unwrap_or(1).saturating_sub(1)) * opts.limit.unwrap_or(50)) as i64;

    // Build a filter-aware query.  We fall back to a broad query when no
    // filters are given because SQLx compile-time macros require static SQL.
    let rows: Result<Vec<MaterialModel>, _> = match (
        &opts.material_type,
        &opts.is_active,
        &opts.search,
    ) {
        // No filters
        (None, None, None) => sqlx::query_as!(
            MaterialModel,
            r#"SELECT id, material_number, name, description, material_type,
                      unit_of_measure, attributes, is_active, created_at, updated_at
               FROM materials
               ORDER BY material_number ASC
               LIMIT $1 OFFSET $2"#,
            limit, offset
        ).fetch_all(db.get_ref()).await,

        // is_active only
        (None, Some(active), None) => sqlx::query_as!(
            MaterialModel,
            r#"SELECT id, material_number, name, description, material_type,
                      unit_of_measure, attributes, is_active, created_at, updated_at
               FROM materials WHERE is_active = $1
               ORDER BY material_number ASC LIMIT $2 OFFSET $3"#,
            active, limit, offset
        ).fetch_all(db.get_ref()).await,

        // material_type only
        (Some(mtype), None, None) => sqlx::query_as!(
            MaterialModel,
            r#"SELECT id, material_number, name, description, material_type,
                      unit_of_measure, attributes, is_active, created_at, updated_at
               FROM materials WHERE material_type = $1
               ORDER BY material_number ASC LIMIT $2 OFFSET $3"#,
            mtype, limit, offset
        ).fetch_all(db.get_ref()).await,

        // search (ilike on number or name)
        (None, None, Some(q)) => {
            let pat = format!("%{}%", q.to_lowercase());
            sqlx::query_as!(
                MaterialModel,
                r#"SELECT id, material_number, name, description, material_type,
                          unit_of_measure, attributes, is_active, created_at, updated_at
                   FROM materials
                   WHERE lower(material_number) LIKE $1 OR lower(name) LIKE $1
                   ORDER BY material_number ASC LIMIT $2 OFFSET $3"#,
                pat, limit, offset
            ).fetch_all(db.get_ref()).await
        }

        // Fallback: fetch all and let the client filter (avoids combinatorial SQL explosion)
        _ => sqlx::query_as!(
            MaterialModel,
            r#"SELECT id, material_number, name, description, material_type,
                      unit_of_measure, attributes, is_active, created_at, updated_at
               FROM materials
               ORDER BY material_number ASC LIMIT $1 OFFSET $2"#,
            limit, offset
        ).fetch_all(db.get_ref()).await,
    };

    match rows {
        Ok(ms) => {
            let items: Vec<MaterialResponse> = ms.iter().map(|m| m.into()).collect();
            HttpResponse::Ok().json(json!({ "status": "success", "results": items.len(), "materials": items }))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

// ── POST /api/materials ───────────────────────────────────────────────────────

#[post("/api/materials")]
pub async fn create_material(
    body: web::Json<CreateMaterialBody>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let id  = Uuid::new_v4();
    let now = Utc::now();

    match sqlx::query_as!(
        MaterialModel,
        r#"INSERT INTO materials
               (id, material_number, name, description, material_type,
                unit_of_measure, attributes, is_active, created_at, updated_at)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
           RETURNING id, material_number, name, description, material_type,
                     unit_of_measure, attributes, is_active, created_at, updated_at"#,
        id,
        body.material_number.trim(),
        body.name.trim(),
        body.description.trim(),
        body.material_type.trim(),
        body.unit_of_measure.trim(),
        body.attributes,
        body.is_active,
        now, now
    ).fetch_one(db.get_ref()).await {
        Ok(m) => HttpResponse::Created()
            .json(json!({ "status": "success", "material": MaterialResponse::from(&m) })),
        Err(e) if e.to_string().contains("duplicate") => HttpResponse::Conflict()
            .json(json!({ "status": "error", "message": "Material number already exists" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

// ── GET /api/materials/:id ────────────────────────────────────────────────────

#[get("/api/materials/{id}")]
pub async fn get_material(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };
    match sqlx::query_as!(
        MaterialModel,
        r#"SELECT id, material_number, name, description, material_type,
                  unit_of_measure, attributes, is_active, created_at, updated_at
           FROM materials WHERE id = $1"#, id
    ).fetch_optional(db.get_ref()).await {
        Ok(Some(m)) => HttpResponse::Ok()
            .json(json!({ "status": "success", "material": MaterialResponse::from(&m) })),
        Ok(None) => HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Material not found" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

// ── PATCH /api/materials/:id ──────────────────────────────────────────────────

#[patch("/api/materials/{id}")]
pub async fn update_material(
    path: web::Path<String>,
    body: web::Json<UpdateMaterialBody>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };

    let cur = match sqlx::query_as!(
        MaterialModel,
        r#"SELECT id, material_number, name, description, material_type,
                  unit_of_measure, attributes, is_active, created_at, updated_at
           FROM materials WHERE id = $1"#, id
    ).fetch_optional(db.get_ref()).await {
        Ok(Some(m)) => m,
        Ok(None) => return HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Material not found" })),
        Err(e) => return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    };

    let now = Utc::now();
    let new_number = body.material_number.as_deref().unwrap_or(&cur.material_number).trim().to_string();
    let new_name   = body.name.as_deref().unwrap_or(&cur.name).trim().to_string();
    let new_desc   = body.description.as_deref().unwrap_or(&cur.description).trim().to_string();
    let new_type   = body.material_type.as_deref().unwrap_or(&cur.material_type).trim().to_string();
    let new_uom    = body.unit_of_measure.as_deref().unwrap_or(&cur.unit_of_measure).trim().to_string();
    let new_attrs  = body.attributes.clone().unwrap_or_else(|| cur.attributes.clone());
    let new_active = body.is_active.unwrap_or(cur.is_active);

    match sqlx::query_as!(
        MaterialModel,
        r#"UPDATE materials
           SET material_number=$1, name=$2, description=$3, material_type=$4,
               unit_of_measure=$5, attributes=$6, is_active=$7, updated_at=$8
           WHERE id = $9
           RETURNING id, material_number, name, description, material_type,
                     unit_of_measure, attributes, is_active, created_at, updated_at"#,
        new_number, new_name, new_desc, new_type, new_uom, new_attrs, new_active, now, id
    ).fetch_one(db.get_ref()).await {
        Ok(m) => HttpResponse::Ok()
            .json(json!({ "status": "success", "material": MaterialResponse::from(&m) })),
        Err(e) if e.to_string().contains("duplicate") => HttpResponse::Conflict()
            .json(json!({ "status": "error", "message": "Material number already exists" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

// ── DELETE /api/materials/:id ─────────────────────────────────────────────────

#[delete("/api/materials/{id}")]
pub async fn delete_material(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };
    match sqlx::query!("DELETE FROM materials WHERE id = $1", id)
        .execute(db.get_ref()).await {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::NoContent().finish(),
        Ok(_) => HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Material not found" })),
        Err(e) if e.to_string().contains("foreign key") => HttpResponse::Conflict()
            .json(json!({ "status": "error", "message": "Cannot delete: batches reference this material" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// LOCATION
// ═══════════════════════════════════════════════════════════════════════════════

#[get("/api/locations")]
pub async fn list_locations(
    opts: web::Query<LocationFilter>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let limit  = opts.limit.unwrap_or(50) as i64;
    let offset = ((opts.page.unwrap_or(1).saturating_sub(1)) * opts.limit.unwrap_or(50)) as i64;

    let rows: Result<Vec<LocationModel>, _> = match (&opts.location_type, &opts.is_active) {
        (None, None) => sqlx::query_as!(
            LocationModel,
            r#"SELECT id, location_code, name, description, location_type,
                      parent_location_id, is_active, created_at, updated_at
               FROM locations ORDER BY location_code ASC LIMIT $1 OFFSET $2"#,
            limit, offset
        ).fetch_all(db.get_ref()).await,

        (Some(ltype), None) => sqlx::query_as!(
            LocationModel,
            r#"SELECT id, location_code, name, description, location_type,
                      parent_location_id, is_active, created_at, updated_at
               FROM locations WHERE location_type = $1
               ORDER BY location_code ASC LIMIT $2 OFFSET $3"#,
            ltype, limit, offset
        ).fetch_all(db.get_ref()).await,

        (None, Some(active)) => sqlx::query_as!(
            LocationModel,
            r#"SELECT id, location_code, name, description, location_type,
                      parent_location_id, is_active, created_at, updated_at
               FROM locations WHERE is_active = $1
               ORDER BY location_code ASC LIMIT $2 OFFSET $3"#,
            active, limit, offset
        ).fetch_all(db.get_ref()).await,

        _ => sqlx::query_as!(
            LocationModel,
            r#"SELECT id, location_code, name, description, location_type,
                      parent_location_id, is_active, created_at, updated_at
               FROM locations ORDER BY location_code ASC LIMIT $1 OFFSET $2"#,
            limit, offset
        ).fetch_all(db.get_ref()).await,
    };

    match rows {
        Ok(ls) => {
            let items: Vec<LocationResponse> = ls.iter().map(|l| l.into()).collect();
            HttpResponse::Ok().json(json!({ "status": "success", "results": items.len(), "locations": items }))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

#[post("/api/locations")]
pub async fn create_location(
    body: web::Json<CreateLocationBody>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let id  = Uuid::new_v4();
    let now = Utc::now();

    let parent_id: Option<Uuid> = match &body.parent_location_id {
        Some(s) if !s.is_empty() => match Uuid::parse_str(s) {
            Ok(u) => Some(u),
            Err(_) => return HttpResponse::BadRequest()
                .json(json!({ "status": "error", "message": "Invalid parent_location_id UUID" })),
        },
        _ => None,
    };

    match sqlx::query_as!(
        LocationModel,
        r#"INSERT INTO locations
               (id, location_code, name, description, location_type,
                parent_location_id, is_active, created_at, updated_at)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
           RETURNING id, location_code, name, description, location_type,
                     parent_location_id, is_active, created_at, updated_at"#,
        id, body.location_code.trim(), body.name.trim(), body.description.trim(),
        body.location_type.trim(), parent_id, body.is_active, now, now
    ).fetch_one(db.get_ref()).await {
        Ok(l) => HttpResponse::Created()
            .json(json!({ "status": "success", "location": LocationResponse::from(&l) })),
        Err(e) if e.to_string().contains("duplicate") => HttpResponse::Conflict()
            .json(json!({ "status": "error", "message": "Location code already exists" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

#[get("/api/locations/{id}")]
pub async fn get_location(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };
    match sqlx::query_as!(
        LocationModel,
        r#"SELECT id, location_code, name, description, location_type,
                  parent_location_id, is_active, created_at, updated_at
           FROM locations WHERE id = $1"#, id
    ).fetch_optional(db.get_ref()).await {
        Ok(Some(l)) => HttpResponse::Ok()
            .json(json!({ "status": "success", "location": LocationResponse::from(&l) })),
        Ok(None) => HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Location not found" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

#[patch("/api/locations/{id}")]
pub async fn update_location(
    path: web::Path<String>,
    body: web::Json<UpdateLocationBody>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };

    let cur = match sqlx::query_as!(
        LocationModel,
        r#"SELECT id, location_code, name, description, location_type,
                  parent_location_id, is_active, created_at, updated_at
           FROM locations WHERE id = $1"#, id
    ).fetch_optional(db.get_ref()).await {
        Ok(Some(l)) => l,
        Ok(None) => return HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Location not found" })),
        Err(e) => return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    };

    let new_parent: Option<Uuid> = match &body.parent_location_id {
        Some(s) if s.is_empty() => None,                        // explicit clear
        Some(s) => match Uuid::parse_str(s) {
            Ok(u) => Some(u),
            Err(_) => return HttpResponse::BadRequest()
                .json(json!({ "status": "error", "message": "Invalid parent_location_id UUID" })),
        },
        None => cur.parent_location_id,                         // unchanged
    };

    let now = Utc::now();
    match sqlx::query_as!(
        LocationModel,
        r#"UPDATE locations
           SET location_code=$1, name=$2, description=$3, location_type=$4,
               parent_location_id=$5, is_active=$6, updated_at=$7
           WHERE id = $8
           RETURNING id, location_code, name, description, location_type,
                     parent_location_id, is_active, created_at, updated_at"#,
        body.location_code.as_deref().unwrap_or(&cur.location_code).trim(),
        body.name.as_deref().unwrap_or(&cur.name).trim(),
        body.description.as_deref().unwrap_or(&cur.description).trim(),
        body.location_type.as_deref().unwrap_or(&cur.location_type).trim(),
        new_parent, body.is_active.unwrap_or(cur.is_active), now, id
    ).fetch_one(db.get_ref()).await {
        Ok(l) => HttpResponse::Ok()
            .json(json!({ "status": "success", "location": LocationResponse::from(&l) })),
        Err(e) if e.to_string().contains("duplicate") => HttpResponse::Conflict()
            .json(json!({ "status": "error", "message": "Location code already exists" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

#[delete("/api/locations/{id}")]
pub async fn delete_location(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };
    match sqlx::query!("DELETE FROM locations WHERE id = $1", id)
        .execute(db.get_ref()).await {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::NoContent().finish(),
        Ok(_) => HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Location not found" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// BATCH
// ═══════════════════════════════════════════════════════════════════════════════

#[get("/api/batches")]
pub async fn list_batches(
    opts: web::Query<BatchFilter>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let limit  = opts.limit.unwrap_or(50) as i64;
    let offset = ((opts.page.unwrap_or(1).saturating_sub(1)) * opts.limit.unwrap_or(50)) as i64;

    let rows: Result<Vec<BatchModel>, _> = match (&opts.material_id, &opts.status) {
        (None, None) => sqlx::query_as!(
            BatchModel,
            r#"SELECT id, batch_number, material_id, material_number, material_name,
                      quantity, unit_of_measure, status, manufactured_date,
                      expiry_date, notes, process_order_id, created_at, updated_at
               FROM batches ORDER BY created_at DESC LIMIT $1 OFFSET $2"#,
            limit, offset
        ).fetch_all(db.get_ref()).await,

        (Some(mid), None) => {
            let mat_uuid = match Uuid::parse_str(mid) {
                Ok(u) => u,
                Err(_) => return HttpResponse::BadRequest()
                    .json(json!({ "status": "error", "message": "Invalid material_id UUID" })),
            };
            sqlx::query_as!(
                BatchModel,
                r#"SELECT id, batch_number, material_id, material_number, material_name,
                          quantity, unit_of_measure, status, manufactured_date,
                          expiry_date, notes, process_order_id, created_at, updated_at
                   FROM batches WHERE material_id = $1
                   ORDER BY created_at DESC LIMIT $2 OFFSET $3"#,
                mat_uuid, limit, offset
            ).fetch_all(db.get_ref()).await
        }

        (None, Some(st)) => sqlx::query_as!(
            BatchModel,
            r#"SELECT id, batch_number, material_id, material_number, material_name,
                      quantity, unit_of_measure, status, manufactured_date,
                      expiry_date, notes, process_order_id, created_at, updated_at
               FROM batches WHERE status = $1
               ORDER BY created_at DESC LIMIT $2 OFFSET $3"#,
            st, limit, offset
        ).fetch_all(db.get_ref()).await,

        _ => sqlx::query_as!(
            BatchModel,
            r#"SELECT id, batch_number, material_id, material_number, material_name,
                      quantity, unit_of_measure, status, manufactured_date,
                      expiry_date, notes, process_order_id, created_at, updated_at
               FROM batches ORDER BY created_at DESC LIMIT $1 OFFSET $2"#,
            limit, offset
        ).fetch_all(db.get_ref()).await,
    };

    match rows {
        Ok(bs) => {
            let items: Vec<BatchResponse> = bs.iter().map(|b| b.into()).collect();
            HttpResponse::Ok().json(json!({ "status": "success", "results": items.len(), "batches": items }))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

#[post("/api/batches")]
pub async fn create_batch(
    body: web::Json<CreateBatchBody>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let mat_uuid = match Uuid::parse_str(&body.material_id) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid material_id UUID" })),
    };

    // Fetch material snapshots
    let mat = match sqlx::query!(
        "SELECT material_number, name FROM materials WHERE id = $1",
        mat_uuid
    ).fetch_optional(db.get_ref()).await {
        Ok(Some(r)) => r,
        Ok(None) => return HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Material not found" })),
        Err(e) => return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    };

    let po_uuid: Option<Uuid> = match &body.process_order_id {
        Some(s) if !s.is_empty() => match Uuid::parse_str(s) {
            Ok(u) => Some(u),
            Err(_) => return HttpResponse::BadRequest()
                .json(json!({ "status": "error", "message": "Invalid process_order_id UUID" })),
        },
        _ => None,
    };

    let id  = Uuid::new_v4();
    let now = Utc::now();
    let qty = sqlx::types::BigDecimal::try_from(body.quantity).unwrap_or_default();

    match sqlx::query_as!(
        BatchModel,
        r#"INSERT INTO batches
               (id, batch_number, material_id, material_number, material_name,
                quantity, unit_of_measure, status, manufactured_date,
                expiry_date, notes, process_order_id, created_at, updated_at)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
           RETURNING id, batch_number, material_id, material_number, material_name,
                     quantity, unit_of_measure, status, manufactured_date,
                     expiry_date, notes, process_order_id, created_at, updated_at"#,
        id, body.batch_number.trim(), mat_uuid, mat.material_number, mat.name,
        qty, body.unit_of_measure.trim(), body.status.trim(),
        body.manufactured_date.trim(), body.expiry_date.trim(), body.notes.trim(),
        po_uuid, now, now
    ).fetch_one(db.get_ref()).await {
        Ok(b) => HttpResponse::Created()
            .json(json!({ "status": "success", "batch": BatchResponse::from(&b) })),
        Err(e) if e.to_string().contains("duplicate") => HttpResponse::Conflict()
            .json(json!({ "status": "error", "message": "Batch number already exists" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

#[get("/api/batches/{id}")]
pub async fn get_batch(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };
    match sqlx::query_as!(
        BatchModel,
        r#"SELECT id, batch_number, material_id, material_number, material_name,
                  quantity, unit_of_measure, status, manufactured_date,
                  expiry_date, notes, process_order_id, created_at, updated_at
           FROM batches WHERE id = $1"#, id
    ).fetch_optional(db.get_ref()).await {
        Ok(Some(b)) => HttpResponse::Ok()
            .json(json!({ "status": "success", "batch": BatchResponse::from(&b) })),
        Ok(None) => HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Batch not found" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

#[patch("/api/batches/{id}")]
pub async fn update_batch(
    path: web::Path<String>,
    body: web::Json<UpdateBatchBody>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };

    let cur = match sqlx::query_as!(
        BatchModel,
        r#"SELECT id, batch_number, material_id, material_number, material_name,
                  quantity, unit_of_measure, status, manufactured_date,
                  expiry_date, notes, process_order_id, created_at, updated_at
           FROM batches WHERE id = $1"#, id
    ).fetch_optional(db.get_ref()).await {
        Ok(Some(b)) => b,
        Ok(None) => return HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Batch not found" })),
        Err(e) => return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    };

    // Resolve material if changed
    let (new_mat_id, new_mat_number, new_mat_name) = if let Some(ref mid) = body.material_id {
        let mat_uuid = match Uuid::parse_str(mid) {
            Ok(u) => u,
            Err(_) => return HttpResponse::BadRequest()
                .json(json!({ "status": "error", "message": "Invalid material_id UUID" })),
        };
        match sqlx::query!("SELECT material_number, name FROM materials WHERE id = $1", mat_uuid)
            .fetch_optional(db.get_ref()).await {
            Ok(Some(r)) => (mat_uuid, r.material_number, r.name),
            Ok(None) => return HttpResponse::NotFound()
                .json(json!({ "status": "error", "message": "Material not found" })),
            Err(e) => return HttpResponse::InternalServerError()
                .json(json!({ "status": "error", "message": format!("{:?}", e) })),
        }
    } else {
        (cur.material_id, cur.material_number.clone(), cur.material_name.clone())
    };

    let new_po: Option<Uuid> = match &body.process_order_id {
        Some(s) if s.is_empty() => None,
        Some(s) => match Uuid::parse_str(s) {
            Ok(u) => Some(u),
            Err(_) => return HttpResponse::BadRequest()
                .json(json!({ "status": "error", "message": "Invalid process_order_id UUID" })),
        },
        None => cur.process_order_id,
    };

    let new_qty = body.quantity
        .map(|q| sqlx::types::BigDecimal::try_from(q).unwrap_or_default())
        .unwrap_or_else(|| cur.quantity.clone());

    let now = Utc::now();
    match sqlx::query_as!(
        BatchModel,
        r#"UPDATE batches
           SET batch_number=$1, material_id=$2, material_number=$3, material_name=$4,
               quantity=$5, unit_of_measure=$6, status=$7, manufactured_date=$8,
               expiry_date=$9, notes=$10, process_order_id=$11, updated_at=$12
           WHERE id = $13
           RETURNING id, batch_number, material_id, material_number, material_name,
                     quantity, unit_of_measure, status, manufactured_date,
                     expiry_date, notes, process_order_id, created_at, updated_at"#,
        body.batch_number.as_deref().unwrap_or(&cur.batch_number).trim(),
        new_mat_id, new_mat_number, new_mat_name, new_qty,
        body.unit_of_measure.as_deref().unwrap_or(&cur.unit_of_measure).trim(),
        body.status.as_deref().unwrap_or(&cur.status).trim(),
        body.manufactured_date.as_deref().unwrap_or(&cur.manufactured_date).trim(),
        body.expiry_date.as_deref().unwrap_or(&cur.expiry_date).trim(),
        body.notes.as_deref().unwrap_or(&cur.notes).trim(),
        new_po, now, id
    ).fetch_one(db.get_ref()).await {
        Ok(b) => HttpResponse::Ok()
            .json(json!({ "status": "success", "batch": BatchResponse::from(&b) })),
        Err(e) if e.to_string().contains("duplicate") => HttpResponse::Conflict()
            .json(json!({ "status": "error", "message": "Batch number already exists" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

#[delete("/api/batches/{id}")]
pub async fn delete_batch(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };
    match sqlx::query!("DELETE FROM batches WHERE id = $1", id)
        .execute(db.get_ref()).await {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::NoContent().finish(),
        Ok(_) => HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Batch not found" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLING UNIT
// ═══════════════════════════════════════════════════════════════════════════════

#[get("/api/handling-units")]
pub async fn list_handling_units(
    opts: web::Query<HuFilter>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let limit  = opts.limit.unwrap_or(50) as i64;
    let offset = ((opts.page.unwrap_or(1).saturating_sub(1)) * opts.limit.unwrap_or(50)) as i64;

    let rows: Result<Vec<HandlingUnitModel>, _> = match (&opts.status, &opts.location_id, &opts.batch_id) {
        (None, None, None) => sqlx::query_as!(
            HandlingUnitModel,
            r#"SELECT id, hu_number, description, hu_type, status,
                      batch_id, batch_number, quantity, unit_of_measure,
                      location_id, location_code, created_at, updated_at
               FROM handling_units ORDER BY hu_number ASC LIMIT $1 OFFSET $2"#,
            limit, offset
        ).fetch_all(db.get_ref()).await,

        (Some(st), None, None) => sqlx::query_as!(
            HandlingUnitModel,
            r#"SELECT id, hu_number, description, hu_type, status,
                      batch_id, batch_number, quantity, unit_of_measure,
                      location_id, location_code, created_at, updated_at
               FROM handling_units WHERE status = $1
               ORDER BY hu_number ASC LIMIT $2 OFFSET $3"#,
            st, limit, offset
        ).fetch_all(db.get_ref()).await,

        _ => sqlx::query_as!(
            HandlingUnitModel,
            r#"SELECT id, hu_number, description, hu_type, status,
                      batch_id, batch_number, quantity, unit_of_measure,
                      location_id, location_code, created_at, updated_at
               FROM handling_units ORDER BY hu_number ASC LIMIT $1 OFFSET $2"#,
            limit, offset
        ).fetch_all(db.get_ref()).await,
    };

    match rows {
        Ok(hs) => {
            let items: Vec<HandlingUnitResponse> = hs.iter().map(|h| h.into()).collect();
            HttpResponse::Ok().json(json!({ "status": "success", "results": items.len(), "handling_units": items }))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

#[post("/api/handling-units")]
pub async fn create_handling_unit(
    body: web::Json<CreateHandlingUnitBody>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let id  = Uuid::new_v4();
    let now = Utc::now();

    // Resolve batch
    let (batch_uuid, batch_number_snap) = match &body.batch_id {
        Some(s) if !s.is_empty() => {
            let u = match Uuid::parse_str(s) {
                Ok(u) => u,
                Err(_) => return HttpResponse::BadRequest()
                    .json(json!({ "status": "error", "message": "Invalid batch_id UUID" })),
            };
            match sqlx::query_scalar!("SELECT batch_number FROM batches WHERE id = $1", u)
                .fetch_optional(db.get_ref()).await {
                Ok(Some(bn)) => (Some(u), bn),
                Ok(None) => return HttpResponse::NotFound()
                    .json(json!({ "status": "error", "message": "Batch not found" })),
                Err(e) => return HttpResponse::InternalServerError()
                    .json(json!({ "status": "error", "message": format!("{:?}", e) })),
            }
        }
        _ => (None, String::new()),
    };

    // Resolve location
    let (loc_uuid, loc_code_snap) = match &body.location_id {
        Some(s) if !s.is_empty() => {
            let u = match Uuid::parse_str(s) {
                Ok(u) => u,
                Err(_) => return HttpResponse::BadRequest()
                    .json(json!({ "status": "error", "message": "Invalid location_id UUID" })),
            };
            match sqlx::query_scalar!("SELECT location_code FROM locations WHERE id = $1", u)
                .fetch_optional(db.get_ref()).await {
                Ok(Some(lc)) => (Some(u), lc),
                Ok(None) => return HttpResponse::NotFound()
                    .json(json!({ "status": "error", "message": "Location not found" })),
                Err(e) => return HttpResponse::InternalServerError()
                    .json(json!({ "status": "error", "message": format!("{:?}", e) })),
            }
        }
        _ => (None, String::new()),
    };

    let qty = sqlx::types::BigDecimal::try_from(body.quantity.unwrap_or(0.0)).unwrap_or_default();

    match sqlx::query_as!(
        HandlingUnitModel,
        r#"INSERT INTO handling_units
               (id, hu_number, description, hu_type, status,
                batch_id, batch_number, quantity, unit_of_measure,
                location_id, location_code, created_at, updated_at)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
           RETURNING id, hu_number, description, hu_type, status,
                     batch_id, batch_number, quantity, unit_of_measure,
                     location_id, location_code, created_at, updated_at"#,
        id, body.hu_number.trim(), body.description.trim(),
        body.hu_type.trim(), body.status.trim(),
        batch_uuid, batch_number_snap, qty, body.unit_of_measure.trim(),
        loc_uuid, loc_code_snap, now, now
    ).fetch_one(db.get_ref()).await {
        Ok(h) => HttpResponse::Created()
            .json(json!({ "status": "success", "handling_unit": HandlingUnitResponse::from(&h) })),
        Err(e) if e.to_string().contains("duplicate") => HttpResponse::Conflict()
            .json(json!({ "status": "error", "message": "HU number already exists" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

#[get("/api/handling-units/{id}")]
pub async fn get_handling_unit(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };
    match sqlx::query_as!(
        HandlingUnitModel,
        r#"SELECT id, hu_number, description, hu_type, status,
                  batch_id, batch_number, quantity, unit_of_measure,
                  location_id, location_code, created_at, updated_at
           FROM handling_units WHERE id = $1"#, id
    ).fetch_optional(db.get_ref()).await {
        Ok(Some(h)) => HttpResponse::Ok()
            .json(json!({ "status": "success", "handling_unit": HandlingUnitResponse::from(&h) })),
        Ok(None) => HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Handling unit not found" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

#[patch("/api/handling-units/{id}")]
pub async fn update_handling_unit(
    path: web::Path<String>,
    body: web::Json<UpdateHandlingUnitBody>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };

    let cur = match sqlx::query_as!(
        HandlingUnitModel,
        r#"SELECT id, hu_number, description, hu_type, status,
                  batch_id, batch_number, quantity, unit_of_measure,
                  location_id, location_code, created_at, updated_at
           FROM handling_units WHERE id = $1"#, id
    ).fetch_optional(db.get_ref()).await {
        Ok(Some(h)) => h,
        Ok(None) => return HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Handling unit not found" })),
        Err(e) => return HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    };

    // Resolve batch
    let (new_batch_id, new_batch_number) = match &body.batch_id {
        Some(s) if s.is_empty() => (None, String::new()),
        Some(s) => {
            let u = match Uuid::parse_str(s) {
                Ok(u) => u,
                Err(_) => return HttpResponse::BadRequest()
                    .json(json!({ "status": "error", "message": "Invalid batch_id UUID" })),
            };
            match sqlx::query_scalar!("SELECT batch_number FROM batches WHERE id = $1", u)
                .fetch_optional(db.get_ref()).await {
                Ok(Some(bn)) => (Some(u), bn),
                Ok(None) => return HttpResponse::NotFound()
                    .json(json!({ "status": "error", "message": "Batch not found" })),
                Err(e) => return HttpResponse::InternalServerError()
                    .json(json!({ "status": "error", "message": format!("{:?}", e) })),
            }
        }
        None => (cur.batch_id, cur.batch_number.clone()),
    };

    // Resolve location
    let (new_loc_id, new_loc_code) = match &body.location_id {
        Some(s) if s.is_empty() => (None, String::new()),
        Some(s) => {
            let u = match Uuid::parse_str(s) {
                Ok(u) => u,
                Err(_) => return HttpResponse::BadRequest()
                    .json(json!({ "status": "error", "message": "Invalid location_id UUID" })),
            };
            match sqlx::query_scalar!("SELECT location_code FROM locations WHERE id = $1", u)
                .fetch_optional(db.get_ref()).await {
                Ok(Some(lc)) => (Some(u), lc),
                Ok(None) => return HttpResponse::NotFound()
                    .json(json!({ "status": "error", "message": "Location not found" })),
                Err(e) => return HttpResponse::InternalServerError()
                    .json(json!({ "status": "error", "message": format!("{:?}", e) })),
            }
        }
        None => (cur.location_id, cur.location_code.clone()),
    };

    let new_qty = body.quantity
        .map(|q| sqlx::types::BigDecimal::try_from(q).unwrap_or_default())
        .unwrap_or_else(|| cur.quantity.clone());

    let now = Utc::now();
    match sqlx::query_as!(
        HandlingUnitModel,
        r#"UPDATE handling_units
           SET hu_number=$1, description=$2, hu_type=$3, status=$4,
               batch_id=$5, batch_number=$6, quantity=$7, unit_of_measure=$8,
               location_id=$9, location_code=$10, updated_at=$11
           WHERE id = $12
           RETURNING id, hu_number, description, hu_type, status,
                     batch_id, batch_number, quantity, unit_of_measure,
                     location_id, location_code, created_at, updated_at"#,
        body.hu_number.as_deref().unwrap_or(&cur.hu_number).trim(),
        body.description.as_deref().unwrap_or(&cur.description).trim(),
        body.hu_type.as_deref().unwrap_or(&cur.hu_type).trim(),
        body.status.as_deref().unwrap_or(&cur.status).trim(),
        new_batch_id, new_batch_number, new_qty,
        body.unit_of_measure.as_deref().unwrap_or(&cur.unit_of_measure).trim(),
        new_loc_id, new_loc_code, now, id
    ).fetch_one(db.get_ref()).await {
        Ok(h) => HttpResponse::Ok()
            .json(json!({ "status": "success", "handling_unit": HandlingUnitResponse::from(&h) })),
        Err(e) if e.to_string().contains("duplicate") => HttpResponse::Conflict()
            .json(json!({ "status": "error", "message": "HU number already exists" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}

#[delete("/api/handling-units/{id}")]
pub async fn delete_handling_unit(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => return HttpResponse::BadRequest()
            .json(json!({ "status": "error", "message": "Invalid UUID" })),
    };
    match sqlx::query!("DELETE FROM handling_units WHERE id = $1", id)
        .execute(db.get_ref()).await {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::NoContent().finish(),
        Ok(_) => HttpResponse::NotFound()
            .json(json!({ "status": "error", "message": "Handling unit not found" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{:?}", e) })),
    }
}
