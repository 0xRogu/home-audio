use actix_web::{web, HttpResponse, Error};
use chrono::Utc;
use jsonwebtoken::{encode, decode, Header, EncodingKey, DecodingKey, Validation};

use crate::models::{LoginRequest, AuthResponse, Claims};
use crate::error::AppError;
use crate::config::AppState;

pub async fn validate_token(token: &str, secret: &str) -> Option<String> {
    let validation = Validation::default();
    match decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &validation
    ) {
        Ok(token_data) => Some(token_data.claims.sub),
        Err(_) => None,
    }
}

pub async fn login(
    req: web::Json<LoginRequest>,
    state: web::Data<AppState>
) -> Result<HttpResponse, AppError> {
    let user = sqlx::query!(
        "SELECT id, is_admin FROM users WHERE username = ? AND password = ?",
        req.username, req.password
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError(e.to_string()))?;

    if let Some(user) = user {
        let claims = Claims {
            sub: user.id,
            exp: (Utc::now() + chrono::Duration::days(1)).timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(state.secret_key.as_ref())
        ).map_err(|e| AppError(e.to_string()))?;

        Ok(HttpResponse::Ok().json(AuthResponse { token }))
    } else {
        Err(AppError("Invalid credentials".to_string()))
    }
}

pub async fn check_admin(user_id: &str, pool: &sqlx::SqlitePool) -> Result<bool, AppError> {
    let is_admin = sqlx::query!(
        "SELECT is_admin FROM users WHERE id = ?",
        user_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError(e.to_string()))?
    .is_admin;
    
    Ok(is_admin)
}
