use rocket::{Responder, serde::json::Json};

use crate::kennel::json::CreatureJson;

#[derive(Responder)]
pub enum Response {
    #[response(status = 200, content_type = "json")]
    Json(Json<Vec<CreatureJson>>),
    #[response(status = 200, content_type = "image/png")]
    Png(Vec<u8>),
    #[response(status = 200, content_type = "image/jpeg")]
    Jpeg(Vec<u8>),
    #[response(status = 200, content_type = "image/gif")]
    Gif(Vec<u8>),
    #[response(status = 200, content_type = "image/webp")]
    Webp(Vec<u8>),
    #[response(status = 500)]
    Err(String),
}

impl Response {
    pub fn from_json(json: Json<Vec<CreatureJson>>) -> Self {
        Self::Json(json)
    }

    pub fn from_png(png: Vec<u8>) -> Self {
        Self::Png(png)
    }

    pub fn from_jpeg(jpeg: Vec<u8>) -> Self {
        Self::Jpeg(jpeg)
    }

    pub fn from_gif(gif: Vec<u8>) -> Self {
        Self::Gif(gif)
    }

    pub fn from_webp(webp: Vec<u8>) -> Self {
        Self::Webp(webp)
    }

    pub fn new_err(message: String) -> Self {
        Self::Err(message)
    }
}
