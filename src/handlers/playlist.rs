use actix_web::{web, Error, HttpRequest, HttpResponse};
use chrono::Utc;
use uuid::Uuid;

use crate::auth::{check_admin, validate_token};
use crate::config::AppState;
use crate::error::AppError;
use crate::models::{
    AddToPlaylistRequest, CreatePlaylistRequest, Playlist, PlaylistAudioItem, PlaylistItem,
    PlaylistWithItems,
};

pub async fn create_playlist(
    req: web::Json<CreatePlaylistRequest>,
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

    let playlist_id = Uuid::new_v4().to_string();
    let created_at = Utc::now();

    sqlx::query!(
        "INSERT INTO playlists (id, name, user_id, created_at) VALUES (?, ?, ?, ?)",
        playlist_id,
        req.name,
        user_id,
        created_at
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError(e.to_string()))?;

    let playlist = Playlist {
        id: playlist_id,
        name: req.name.clone(),
        user_id,
        created_at,
    };

    Ok(HttpResponse::Ok().json(playlist))
}

pub async fn get_playlists(
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

    let playlists = if is_admin {
        // Admins can see all playlists
        sqlx::query_as::<_, Playlist>("SELECT * FROM playlists ORDER BY created_at DESC")
            .fetch_all(&state.db_pool)
            .await
    } else {
        // Regular users can only see their own playlists
        sqlx::query_as::<_, Playlist>(
            "SELECT * FROM playlists WHERE user_id = ? ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&state.db_pool)
        .await
    }
    .map_err(|e| AppError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(playlists))
}

pub async fn get_playlist(
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

    let user_id = validate_token(token, &state.secret_key)
        .await
        .ok_or_else(|| AppError("Invalid token".to_string()))?;

    // Check if user is admin
    let is_admin = check_admin(&user_id, &state.db_pool).await?;

    let playlist_id = path.into_inner();
    let playlist = sqlx::query_as::<_, Playlist>("SELECT * FROM playlists WHERE id = ?")
        .bind(&playlist_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError(e.to_string()))?;

    if let Some(playlist) = playlist {
        // Check if user has access to this playlist
        if playlist.user_id != user_id && !is_admin {
            return Err(AppError("Not authorized to access this playlist".to_string()).into());
        }

        // Get playlist items with audio details
        let items = sqlx::query!(
            "SELECT pi.id, pi.audio_id, pi.position, af.filename, af.mime_type 
             FROM playlist_items pi
             JOIN audio_files af ON pi.audio_id = af.id
             WHERE pi.playlist_id = ?
             ORDER BY pi.position",
            playlist_id
        )
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| AppError(e.to_string()))?;

        let playlist_items: Vec<PlaylistAudioItem> = items
            .into_iter()
            .map(|item| PlaylistAudioItem {
                id: item.id.expect("Item ID should not be null"),
                audio_id: item.audio_id,
                position: item.position as i32,
                filename: item.filename,
                mime_type: item.mime_type,
            })
            .collect();

        let playlist_with_items = PlaylistWithItems {
            id: playlist.id,
            name: playlist.name,
            user_id: playlist.user_id,
            created_at: playlist.created_at,
            items: playlist_items,
        };

        Ok(HttpResponse::Ok().json(playlist_with_items))
    } else {
        Err(AppError("Playlist not found".to_string()).into())
    }
}

pub async fn delete_playlist(
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

    let user_id = validate_token(token, &state.secret_key)
        .await
        .ok_or_else(|| AppError("Invalid token".to_string()))?;

    // Check if user is admin
    let is_admin = check_admin(&user_id, &state.db_pool).await?;

    let playlist_id = path.into_inner();
    let playlist = sqlx::query_as::<_, Playlist>("SELECT * FROM playlists WHERE id = ?")
        .bind(&playlist_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError(e.to_string()))?;

    if let Some(playlist) = playlist {
        // Check if user has access to delete this playlist
        if playlist.user_id != user_id && !is_admin {
            return Err(AppError("Not authorized to delete this playlist".to_string()).into());
        }

        // First delete all playlist items
        sqlx::query!(
            "DELETE FROM playlist_items WHERE playlist_id = ?",
            playlist_id
        )
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError(e.to_string()))?;

        // Then delete the playlist
        sqlx::query!("DELETE FROM playlists WHERE id = ?", playlist_id)
            .execute(&state.db_pool)
            .await
            .map_err(|e| AppError(e.to_string()))?;

        Ok(HttpResponse::Ok().body("Playlist deleted"))
    } else {
        Err(AppError("Playlist not found".to_string()).into())
    }
}

pub async fn add_to_playlist(
    path: web::Path<String>,
    req: web::Json<AddToPlaylistRequest>,
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

    let playlist_id = path.into_inner();

    // Check if playlist exists and user has access
    let playlist = sqlx::query_as::<_, Playlist>("SELECT * FROM playlists WHERE id = ?")
        .bind(&playlist_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError(e.to_string()))?;

    if let Some(playlist) = playlist {
        // Check if user owns this playlist
        if playlist.user_id != user_id {
            return Err(AppError("Not authorized to modify this playlist".to_string()).into());
        }

        // Check if audio file exists
        let audio = sqlx::query!("SELECT id FROM audio_files WHERE id = ?", req.audio_id)
            .fetch_optional(&state.db_pool)
            .await
            .map_err(|e| AppError(e.to_string()))?;

        if audio.is_none() {
            return Err(AppError("Audio file not found".to_string()).into());
        }

        // Determine position
        let position = if let Some(pos) = req.position {
            pos
        } else {
            // Get the highest position and add 1
            let max_position = sqlx::query!(
                "SELECT MAX(position) as max_pos FROM playlist_items WHERE playlist_id = ?",
                playlist_id
            )
            .fetch_one(&state.db_pool)
            .await
            .map_err(|e| AppError(e.to_string()))?
            .max_pos
            .unwrap_or(0);

            (max_position + 1) as i32
        };

        let item_id = Uuid::new_v4().to_string();

        sqlx::query!(
            "INSERT INTO playlist_items (id, playlist_id, audio_id, position) VALUES (?, ?, ?, ?)",
            item_id,
            playlist_id,
            req.audio_id,
            position
        )
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError(e.to_string()))?;

        let item = PlaylistItem {
            id: item_id,
            playlist_id,
            audio_id: req.audio_id.clone(),
            position,
        };

        Ok(HttpResponse::Ok().json(item))
    } else {
        Err(AppError("Playlist not found".to_string()).into())
    }
}

pub async fn remove_from_playlist(
    path: web::Path<(String, String)>,
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

    let (playlist_id, item_id) = path.into_inner();

    // Check if playlist exists and user has access
    let playlist = sqlx::query_as::<_, Playlist>("SELECT * FROM playlists WHERE id = ?")
        .bind(&playlist_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError(e.to_string()))?;

    if let Some(playlist) = playlist {
        // Check if user owns this playlist
        if playlist.user_id != user_id {
            return Err(AppError("Not authorized to modify this playlist".to_string()).into());
        }

        // Check if item exists in the playlist
        let item = sqlx::query!(
            "SELECT id FROM playlist_items WHERE id = ? AND playlist_id = ?",
            item_id,
            playlist_id
        )
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError(e.to_string()))?;

        if item.is_none() {
            return Err(AppError("Item not found in playlist".to_string()).into());
        }

        // Delete the item
        sqlx::query!("DELETE FROM playlist_items WHERE id = ?", item_id)
            .execute(&state.db_pool)
            .await
            .map_err(|e| AppError(e.to_string()))?;

        Ok(HttpResponse::Ok().body("Item removed from playlist"))
    } else {
        Err(AppError("Playlist not found".to_string()).into())
    }
}
