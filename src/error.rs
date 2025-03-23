use actix_web::{http::StatusCode, HttpResponse};

#[derive(Debug)]
pub struct AppError(pub String);

impl actix_web::error::ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.0.clone())
    }
}
