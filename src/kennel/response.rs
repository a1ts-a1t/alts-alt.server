use kennel_club::ImageFormat;
use rocket::{
    Responder,
    http::{self, ContentType},
};
use serde::Serialize;

#[derive(Responder)]
pub enum Response {
    #[response(status = 200)]
    Json(String, ContentType),
    #[response(status = 200)]
    Image(Vec<u8>, ContentType),
    Err {
        inner: (http::Status, String),
    },
}

impl Response {
    pub fn new_json<T: Serialize>(json: T) -> Self {
        match serde_json::to_string(&json) {
            Ok(s) => Self::Json(s, ContentType::JSON),
            Err(e) => Self::Err {
                inner: (http::Status::InternalServerError, e.to_string()),
            },
        }
    }

    pub fn new_image(data: Vec<u8>, format: ImageFormat) -> Self {
        let content_type = ContentType::parse_flexible(format.to_mime_type())
            .expect("Error parsing image content type");
        Self::Image(data, content_type)
    }

    pub fn new_err(status: http::Status, message: &str) -> Self {
        Self::Err {
            inner: (status, message.to_string()),
        }
    }
}
