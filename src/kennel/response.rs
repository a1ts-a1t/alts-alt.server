use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response as AxumResponse};
use kennel_club::ImageFormat;
use serde::Serialize;

const NO_CACHE: &str = "no-cache, no-store";
const IMMUTABLE: &str = "public, max-age=31536000, immutable";

pub enum Response {
    Json(String),
    Image(Vec<u8>, &'static str),
    CachedImage(Vec<u8>, &'static str),
    Err { status: StatusCode, message: String },
    PermanentRedirect(String),
    TemporaryRedirect(String),
}

impl Response {
    pub fn new_json<T: Serialize>(json: T) -> Self {
        match serde_json::to_string(&json) {
            Ok(s) => Self::Json(s),
            Err(e) => Self::Err {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: e.to_string(),
            },
        }
    }

    pub fn new_image(data: Vec<u8>, format: ImageFormat) -> Self {
        Self::Image(data, format.to_mime_type())
    }

    pub fn new_cached_image(data: Vec<u8>, format: ImageFormat) -> Self {
        Self::CachedImage(data, format.to_mime_type())
    }

    pub fn new_err(status: StatusCode, message: &str) -> Self {
        Self::Err {
            status,
            message: message.to_string(),
        }
    }

    pub fn new_permanent_redirect(location: String) -> Self {
        Self::PermanentRedirect(location)
    }

    pub fn new_temporary_redirect(location: String) -> Self {
        Self::TemporaryRedirect(location)
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> AxumResponse {
        match self {
            Self::Json(body) => (
                [
                    (header::CONTENT_TYPE, "application/json"),
                    (header::CACHE_CONTROL, NO_CACHE),
                ],
                body,
            )
                .into_response(),
            Self::Image(body, mime) => (
                [
                    (header::CONTENT_TYPE, mime),
                    (header::CACHE_CONTROL, NO_CACHE),
                ],
                body,
            )
                .into_response(),
            Self::CachedImage(body, mime) => (
                [
                    (header::CONTENT_TYPE, mime),
                    (header::CACHE_CONTROL, IMMUTABLE),
                ],
                body,
            )
                .into_response(),
            Self::Err { status, message } => (status, message).into_response(),
            Self::PermanentRedirect(location) => (
                StatusCode::MOVED_PERMANENTLY,
                [(header::LOCATION, location)],
            )
                .into_response(),
            Self::TemporaryRedirect(location) => {
                (StatusCode::FOUND, [(header::LOCATION, location)]).into_response()
            }
        }
    }
}
