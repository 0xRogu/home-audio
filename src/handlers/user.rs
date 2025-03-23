use actix_web::{web, Error, HttpRequest, HttpResponse};
use sqlx::Row;
use std::fs;
use uuid::Uuid;

use crate::auth::{check_admin, validate_token};
use crate::config::AppState;
use crate::error::AppError;
use crate::models::{CreateUserRequest, UserResponse};

pub async fn create_user(
    req: web::Json<CreateUserRequest>,
    state: web::Data<AppState>,
    http_req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let token = http_req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| AppError("Authentication required".to_string()))?;

    let user_id = validate_token(token, &state.secret_key)
        .await
        .ok_or_else(|| AppError("Invalid token".to_string()))?;

    // Check if user is admin
    let is_admin = check_admin(&user_id, &state.db_pool).await?;
    if !is_admin {
        return Err(AppError("Only admin users can create new users".to_string()).into());
    }

    // Check if username already exists
    let existing_user = sqlx::query("SELECT id FROM users WHERE username = ?")
        .bind(&req.username)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError(e.to_string()))?;

    if existing_user.is_some() {
        return Err(AppError("Username already exists".to_string()).into());
    }

    let new_user_id = Uuid::new_v4().to_string();

    // Create user folder
    let user_folder = format!("./uploads/{}", new_user_id);
    fs::create_dir_all(&user_folder)?;

    // Create user in database
    sqlx::query(
        "INSERT INTO users (id, username, password, is_admin, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&new_user_id)
    .bind(&req.username)
    .bind(&req.password)
    .bind(req.is_admin)
    .bind(chrono::Utc::now())
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError(e.to_string()))?;

    let user_response = UserResponse {
        id: new_user_id,
        username: req.username.clone(),
        is_admin: req.is_admin,
    };

    Ok(HttpResponse::Ok().json(user_response))
}

pub async fn delete_user(
    path: web::Path<String>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| AppError("Authentication required".to_string()))?;

    let admin_id = validate_token(token, &state.secret_key)
        .await
        .ok_or_else(|| AppError("Invalid token".to_string()))?;

    // Check if user is admin
    let is_admin = check_admin(&admin_id, &state.db_pool).await?;
    if !is_admin {
        return Err(AppError("Only admin users can delete users".to_string()).into());
    }

    let user_id = path.into_inner();

    // Prevent admin from deleting their own account
    if user_id == admin_id {
        return Err(AppError("Cannot delete your own account".to_string()).into());
    }

    // Check if user exists
    let user = sqlx::query("SELECT id FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError(e.to_string()))?;

    if user.is_none() {
        return Err(AppError("User not found".to_string()).into());
    }

    // Start transaction
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError(e.to_string()))?;

    // Get all audio files for this user
    let audio_files =
        sqlx::query("SELECT id, user_folder, filename FROM audio_files WHERE user_id = ?")
            .bind(&user_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| AppError(e.to_string()))?;

    // Delete playlist items that reference this user's audio files
    for audio in &audio_files {
        sqlx::query("DELETE FROM playlist_items WHERE audio_id = ?")
            .bind(audio.get::<String, _>("id"))
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError(e.to_string()))?;
    }

    // Delete all audio files from the database
    sqlx::query("DELETE FROM audio_files WHERE user_id = ?")
        .bind(&user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError(e.to_string()))?;

    // Delete playlist items in this user's playlists
    let playlists = sqlx::query("SELECT id FROM playlists WHERE user_id = ?")
        .bind(&user_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| AppError(e.to_string()))?;

    for playlist in &playlists {
        sqlx::query("DELETE FROM playlist_items WHERE playlist_id = ?")
            .bind(playlist.get::<String, _>("id"))
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError(e.to_string()))?;
    }

    // Delete all playlists
    sqlx::query("DELETE FROM playlists WHERE user_id = ?")
        .bind(&user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError(e.to_string()))?;

    // Delete the user
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(&user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError(e.to_string()))?;

    // Commit transaction
    tx.commit().await.map_err(|e| AppError(e.to_string()))?;

    // Delete audio files from filesystem
    for audio in audio_files {
        let filepath = format!(
            "{}/{}_{}",
            audio.get::<String, _>("user_folder"),
            audio.get::<String, _>("id"),
            audio.get::<String, _>("filename")
        );
        let _ = fs::remove_file(filepath); // Ignore errors if file doesn't exist
    }

    // Delete user folder
    let user_folder = format!("./uploads/{}", user_id);
    let _ = fs::remove_dir_all(user_folder); // Ignore errors if directory doesn't exist

    Ok(HttpResponse::Ok().body("User deleted"))
}

pub async fn list_users(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| AppError("Authentication required".to_string()))?;

    let user_id = validate_token(token, &state.secret_key)
        .await
        .ok_or_else(|| AppError("Invalid token".to_string()))?;

    // Check if user is admin
    let is_admin = check_admin(&user_id, &state.db_pool).await?;
    if !is_admin {
        return Err(AppError("Only admin users can list all users".to_string()).into());
    }

    // Get all users
    let users = sqlx::query("SELECT id, username, is_admin FROM users")
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| AppError(e.to_string()))?;

    let user_responses: Vec<UserResponse> = users
        .into_iter()
        .map(|user| {
            let row: sqlx::sqlite::SqliteRow = user;
            UserResponse {
                id: row.get("id"),
                username: row.get("username"),
                is_admin: row.get("is_admin"),
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(user_responses))
}
