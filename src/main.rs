use actix_web::{web, App, HttpServer, middleware};
use actix_ratelimit::{RateLimiter, MemoryStore, MemoryStoreActor};
use std::time::Duration;
use std::env;
use sqlx::sqlite::SqlitePoolOptions;
use std::fs;
use dotenv::dotenv;

mod models;
mod error;
mod config;
mod auth;
mod handlers;

use crate::config::{AppState, init_db, load_rustls_config, ensure_ssl_cert_exists};
use crate::auth::login;
use crate::handlers::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Create uploads directory if it doesn't exist
    fs::create_dir_all("./uploads")?;

    // Generate SSL certificates if they don't exist
    ensure_ssl_cert_exists()?;

    // Get secret key from environment variable or use a default
    let secret_key = env::var("SECRET_KEY").unwrap_or_else(|_| "your_secret_key".to_string());

    // Set up database connection pool
    let db_pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite:audio.db")
        .await
        .expect("Failed to create pool");

    // Initialize database tables
    init_db(&db_pool).await.expect("Failed to initialize database");

    // Set up rate limiter
    let store = MemoryStore::new();

    // Create the app state
    let app_state = web::Data::new(AppState {
        db_pool,
        secret_key,
    });

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(
                RateLimiter::new(
                    MemoryStoreActor::from(store.clone()).start())
                    .with_interval(Duration::from_secs(60))
                    .with_max_requests(100)
            )
            .app_data(app_state.clone())
            .route("/login", web::post().to(login))
            .route("/audio", web::post().to(upload_audio))
            .route("/audio/{id}", web::get().to(stream_audio))
            .route("/audio/{id}", web::delete().to(delete_audio))
            .route("/users/{id}/audio", web::get().to(get_user_audio))
            .route("/playlists", web::post().to(create_playlist))
            .route("/playlists", web::get().to(get_playlists))
            .route("/playlists/{id}", web::get().to(get_playlist))
            .route("/playlists/{id}", web::delete().to(delete_playlist))
            .route("/playlists/{id}/items", web::post().to(add_to_playlist))
            .route("/playlists/{id}/items/{item_id}", web::delete().to(remove_from_playlist))
            .route("/users", web::post().to(create_user))
            .route("/users", web::get().to(list_users))
            .route("/users/{id}", web::delete().to(delete_user))
    })
    .bind_rustls("127.0.0.1:8443", load_rustls_config())?
    .run()
    .await
}
