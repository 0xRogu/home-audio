use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AudioFile {
    pub id: String,
    pub filename: String,
    pub user_id: String,
    pub created_at: chrono::DateTime<Utc>,
    pub mime_type: String,
    pub user_folder: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password: String,
    pub is_admin: bool,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub user_id: String,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct PlaylistItem {
    pub id: String,
    pub playlist_id: String,
    pub audio_id: String,
    pub position: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePlaylistRequest {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlaylistWithItems {
    pub id: String,
    pub name: String,
    pub user_id: String,
    pub created_at: chrono::DateTime<Utc>,
    pub items: Vec<PlaylistAudioItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlaylistAudioItem {
    pub id: String,
    pub audio_id: String,
    pub position: i32,
    pub filename: String,
    pub mime_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddToPlaylistRequest {
    pub audio_id: String,
    pub position: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub is_admin: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub is_admin: bool,
}
