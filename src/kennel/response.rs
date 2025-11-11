use kennel_club::ImageFormat;
use rocket::{
    Responder,
    http::{self, ContentType, Header},
};
use serde::Serialize;

#[derive(Responder)]
pub enum Response {
    #[response(status = 200)]
    Json(String, ContentType, Header<'static>),
    #[response(status = 200)]
    Image(Vec<u8>, ContentType, Header<'static>),
    #[response(status = 200)]
    CachedImage(Vec<u8>, ContentType),
    Err {
        inner: (http::Status, String),
    },
    #[response(status = 301)]
    PermanentRedirect((), Header<'static>),
    #[response(status = 302)]
    TemporaryRedirect((), Header<'static>),
}

impl Response {
    pub fn new_json<T: Serialize>(json: T) -> Self {
        let no_cache = Header::new("Cache-Control", "no-cache, no-store");
        match serde_json::to_string(&json) {
            Ok(s) => Self::Json(s, ContentType::JSON, no_cache),
            Err(e) => Self::Err {
                inner: (http::Status::InternalServerError, e.to_string()),
            },
        }
    }

    pub fn new_image(data: Vec<u8>, format: ImageFormat) -> Self {
        let no_cache = Header::new("Cache-Control", "no-cache, no-store");
        let content_type = ContentType::parse_flexible(format.to_mime_type())
            .expect("Error parsing image content type");
        Self::Image(data, content_type, no_cache)
    }

    pub fn new_cached_image(data: Vec<u8>, format: ImageFormat) -> Self {
        let content_type = ContentType::parse_flexible(format.to_mime_type())
            .expect("Error parsing image content type");
        Self::CachedImage(data, content_type)
    }

    pub fn new_err(status: http::Status, message: &str) -> Self {
        Self::Err {
            inner: (status, message.to_string()),
        }
    }

    pub fn new_permanent_redirect(location: String) -> Self {
        let location = Header::new("Location", location);
        Self::PermanentRedirect((), location)
    }

    pub fn new_temporary_redirect(location: String) -> Self {
        let location = Header::new("Location", location);
        Self::TemporaryRedirect((), location)
    }
}
