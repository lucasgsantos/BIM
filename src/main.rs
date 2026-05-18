// src/main.rs

mod handler;
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

    // ── Logging ───────────────────────────────────────────────────────────────
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "actix_web=info");
    }
    env_logger::init();

    // ── Database pool ─────────────────────────────────────────────────────────
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env");

    let pool = match PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
    {
        Ok(p) => {
            println!("✅ Connected to PostgreSQL");
            p
        }
        Err(e) => {
            eprintln!("❌ Failed to connect to database: {:?}", e);
            std::process::exit(1);
        }
    };

    println!("🚀 Workflow Designer API running at http://localhost:8000");

    // ── HTTP server ───────────────────────────────────────────────────────────
    HttpServer::new(move || {
        // CORS — allow the React dev server on :3000 and any origin in prod.
        let cors = Cors::default()
            .allowed_origin("http://localhost:3000")
            .allowed_origin("http://127.0.0.1:3000")
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                header::ACCEPT,
            ])
            .supports_credentials()
            .max_age(3600);

        App::new()
            // Share the DB pool with every handler
            .app_data(web::Data::new(pool.clone()))
            // JSON extractor config — return 400 on bad JSON
            .app_data(
                web::JsonConfig::default()
                    .error_handler(|err, _req| {
                        let response = actix_web::HttpResponse::BadRequest().json(
                            serde_json::json!({
                                "status": "error",
                                "message": format!("{}", err)
                            }),
                        );
                        actix_web::error::InternalError::from_response(err, response).into()
                    }),
            )
            .wrap(cors)
            .wrap(Logger::default())
            // ── Routes ────────────────────────────────────────────────────────
            .service(handler::health_checker)
            .service(handler::list_workflows)
            .service(handler::create_workflow)
            .service(handler::get_workflow)
            .service(handler::save_diagram)
            .service(handler::delete_workflow)
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await
}
