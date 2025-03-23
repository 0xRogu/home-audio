use actix_web::{web, HttpResponse, Error, HttpRequest};
use actix_multipart::Multipart;
use actix_files::NamedFile;
use futures::StreamExt;
use mime::Mime;
use uuid::Uuid;
use chrono::Utc;
use std::fs;
use std::io::Write;

use crate::models::AudioFile;
use crate::error::AppError;
use crate::config::AppState;
use crate::auth::{validate_token, check_admin};

pub async fn upload_audio(
    mut payload: Multipart,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let token = req.headers().get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| AppError("Authentication required".to_string()))?;

    let user_id = validate_token(token, &state.secret_key)
        .await
        .ok_or_else(|| AppError("Invalid token".to_string()))?;

    // Ensure user folder exists
    let user_folder = format!("./uploads/{}", user_id);
    fs::create_dir_all(&user_folder)?;

    while let Some(Ok(mut field)) = payload.next().await {
        let content_type = field.content_type();
        let valid_types = vec![
            mime::AUDIO_MPEG,    // MP3
            mime::AUDIO_WAV,     // WAV
            mime::AUDIO_FLAC,    // FLAC
            mime::AUDIO_AAC,     // AAC
            "audio/ogg".parse::<Mime>().unwrap(), // OGG
        ];

        let mime_type = content_type.ok_or_else(||
            AppError("No content type specified".to_string())
        )?;

        if !valid_types.contains(&mime_type) {
            return Err(AppError("Invalid audio format (only MP3/WAV/FLAC/AAC/OGG)".to_string()).into());
        }

        let filename = field.content_disposition()
            .get_filename()
            .unwrap_or("audio")
            .to_string();
        let audio_id = Uuid::new_v4().to_string();
        let filepath = format!("{}/{}_{}", user_folder, audio_id, filename);

        let mut f = fs::File::create(&filepath)?;
        while let Some(chunk) = field.next().await {
            let data = chunk?;
            f.write_all(&data)?;
        }

        let audio_file = AudioFile {
            id: audio_id.clone(),
            filename,
            user_id: user_id.clone(),
            created_at: Utc::now(),
            mime_type: mime_type.to_string(),
            user_folder,
        };

        sqlx::query(
            "INSERT INTO audio_files (id, filename, user_id, created_at, mime_type, user_folder) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&audio_file.id)
        .bind(&audio_file.filename)
        .bind(&audio_file.user_id)
        .bind(&audio_file.created_at)
        .bind(&audio_file.mime_type)
        .bind(&audio_file.user_folder)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError(e.to_string()))?;

        return Ok(HttpResponse::Ok().json(audio_file));
    }

    Err(AppError("No file uploaded".to_string()).into())
}

pub async fn stream_audio(
    path: web::Path<String>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<NamedFile, Error> {
    let token = req.headers().get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| AppError("Authentication required".to_string()))?;

    let user_id = validate_token(token, &state.secret_key)
        .await
        .ok_or_else(|| AppError("Invalid token".to_string()))?;

    // Check if user is admin
    let is_admin = check_admin(&user_id, &state.db_pool).await?;

    let audio_id = path.into_inner();
    let audio = sqlx::query_as::<_, AudioFile>(
        "SELECT * FROM audio_files WHERE id = ?"
    )
    .bind(&audio_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError(e.to_string()))?;

    if let Some(audio) = audio {
        // Check if user has access to this audio file
        if audio.user_id != user_id && !is_admin {
            return Err(AppError("Not authorized to access this audio file".to_string()).into());
        }

        let filepath = format!("{}/{}_{}", audio.user_folder, audio.id, audio.filename);
        let mime_type = audio.mime_type.parse::<Mime>()
            .unwrap_or(mime::AUDIO_MPEG);
        let file = NamedFile::open(filepath)?
            .set_content_type(mime_type);
        Ok(file)
    } else {
        Err(AppError("Audio not found".to_string()).into())
    }
}

pub async fn delete_audio(
    path: web::Path<String>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let token = req.headers().get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| AppError("Authentication required".to_string()))?;

    let user_id = validate_token(token, &state.secret_key)
        .await
        .ok_or_else(|| AppError("Invalid token".to_string()))?;

    // Check if user is admin
    let is_admin = check_admin(&user_id, &state.db_pool).await?;

    let audio_id = path.into_inner();
    let audio = sqlx::query_as::<_, AudioFile>(
        "SELECT * FROM audio_files WHERE id = ?"
    )
    .bind(&audio_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError(e.to_string()))?;

    if let Some(audio) = audio {
        // Check if user has access to delete this audio file
        if audio.user_id != user_id && !is_admin {
            return Err(AppError("Not authorized to delete this audio file".to_string()).into());
        }

        let filepath = format!("{}/{}_{}", audio.user_folder, audio.id, audio.filename);
        fs::remove_file(filepath)?;

        sqlx::query("DELETE FROM audio_files WHERE id = ?")
            .bind(audio_id)
            .execute(&state.db_pool)
            .await
            .map_err(|e| AppError(e.to_string()))?;

        Ok(HttpResponse::Ok().body("Audio deleted"))
    } else {
        Err(AppError("Audio not found".to_string()).into())
    }
}

pub async fn get_user_audio(
    path: web::Path<String>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let token = req.headers().get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| AppError("Authentication required".to_string()))?;

    let current_user_id = validate_token(token, &state.secret_key)
        .await
        .ok_or_else(|| AppError("Invalid token".to_string()))?;

    let target_user_id = path.into_inner();
    
    // Check if current user is admin or is accessing their own files
    if current_user_id != target_user_id {
        let is_admin = check_admin(&current_user_id, &state.db_pool).await?;
        
        if !is_admin {
            return Err(AppError("Not authorized to access this user's files".to_string()).into());
        }
    }
    
    // Get user's audio files
    let audio_files = sqlx::query_as::<_, AudioFile>(
        "SELECT * FROM audio_files WHERE user_id = ? ORDER BY created_at DESC"
    )
    .bind(target_user_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError(e.to_string()))?;
    
    Ok(HttpResponse::Ok().json(audio_files))
}
