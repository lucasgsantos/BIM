// src/main.rs

mod handler;
mod handler_master;
mod model;
mod schema;

use actix_cors::Cors;
use actix_web::{http::header, middleware::Logger, web, App, HttpServer};
use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "actix_web=info");
    }
    env_logger::init();

    // â”€â”€ Database pool â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env");

    let pool = match PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
    {
        Ok(p) => {
            println!("âœ… Connected to PostgreSQL");
            p
        }
        Err(e) => {
            eprintln!("âŒ Failed to connect to database: {:?}", e);
            std::process::exit(1);
        }
    };

    let api_url = env::var("API_URL")
    .expect("API_URL must be set in .env");

    let api_host = env::var("API_HOST")
    .expect("API_HOST must be set in .env");

    let api_port: u16 = env::var("API_PORT").expect("API_PORT must be set in .env").parse::<u16>().unwrap();

    let web_url = env::var("WEB_URL")
    .expect("WEB_URL must be set in .env");

    println!("BIM MES API running at {:?}", api_url);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&web_url)
            .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                header::ACCEPT,
            ])
            .supports_credentials()
            .max_age(3600);

        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(
                web::JsonConfig::default().error_handler(|err, _req| {
                    let resp = actix_web::HttpResponse::BadRequest().json(
                        serde_json::json!({ "status": "error", "message": format!("{}", err) }),
                    );
                    actix_web::error::InternalError::from_response(err, resp).into()
                }),
            )
            .wrap(cors)
            .wrap(Logger::default())
            // ── Health ────────────────────────────────────────────────────────
            .service(handler::health_checker)
            // ── Workflow (MBR) ────────────────────────────────────────────────
            .service(handler::list_workflows)
            .service(handler::create_workflow)
            .service(handler::get_workflow)
            .service(handler::save_diagram)
            .service(handler::delete_workflow)
            // ── Process Order (EBR) ───────────────────────────────────────────
            .service(handler::list_orders)
            .service(handler::create_order)
            .service(handler::get_order)
            .service(handler::update_order)
            .service(handler::delete_order)
            .service(handler::start_order)
            .service(handler::confirm_step)
            .service(handler::complete_order)
            .service(handler::cancel_order)
            // ── Master Data: Material ─────────────────────────────────────────
            .service(handler_master::list_materials)
            .service(handler_master::create_material)
            .service(handler_master::get_material)
            .service(handler_master::update_material)
            .service(handler_master::delete_material)
            // ── Master Data: Location ─────────────────────────────────────────
            .service(handler_master::list_locations)
            .service(handler_master::create_location)
            .service(handler_master::get_location)
            .service(handler_master::update_location)
            .service(handler_master::delete_location)
            // ── Master Data: Batch ────────────────────────────────────────────
            .service(handler_master::list_batches)
            .service(handler_master::create_batch)
            .service(handler_master::get_batch)
            .service(handler_master::update_batch)
            .service(handler_master::delete_batch)
            // ── Master Data: Handling Unit ────────────────────────────────────
            .service(handler_master::list_handling_units)
            .service(handler_master::create_handling_unit)
            .service(handler_master::get_handling_unit)
            .service(handler_master::update_handling_unit)
            .service(handler_master::delete_handling_unit)
    })
    .bind((api_host, api_port))?
    .run()
    .await
}
