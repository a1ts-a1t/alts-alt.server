use bytes::Bytes;

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{
        HeaderMap, HeaderValue,
        header::{
            CACHE_CONTROL, CONNECTION, CONTENT_LENGTH, ETAG, IF_NONE_MATCH, TRANSFER_ENCODING,
        },
    },
    response::Response,
    routing::get,
};
use hyper::StatusCode;
use moka::future::Cache;

use crate::reverse_proxy::{ReverseProxyConfig, ReverseProxyError, reverse_proxy};

#[derive(Clone)]
struct WebsiteState {
    image_cache: Cache<String, Response<Bytes>>,
    reverse_proxy_config: ReverseProxyConfig,
}

/// Upper bound for buffering upstream image bodies
const MAX_IMAGE_BODY: usize = 2 * 1024 * 1024;

/// Memory budget for cached /_image responses.
const IMAGE_CACHE_MAX_BYTES: u64 = 64 * 1024 * 1024;

pub fn website_router() -> Router {
    let website_origin =
        std::env::var("WEBSITE_ORIGIN").unwrap_or_else(|_| "http://0.0.0.0:4321".to_string());

    let image_cache: Cache<String, Response<Bytes>> = Cache::builder()
        .max_capacity(IMAGE_CACHE_MAX_BYTES)
        // size of each cache entry a little more than the size of each cached response body
        .weigher(|_key, response: &Response<Bytes>| {
            u32::try_from(response.body().len() + 512).unwrap_or(u32::MAX)
        })
        .build();

    let website_state = WebsiteState {
        reverse_proxy_config: ReverseProxyConfig::new(website_origin),
        image_cache,
    };

    Router::new()
        .route("/_image", get(website_image))
        .fallback(website_proxy)
        .with_state(website_state)
}

async fn website_proxy(
    State(website_state): State<WebsiteState>,
    req: Request,
) -> Result<Response, ReverseProxyError> {
    reverse_proxy(website_state.reverse_proxy_config, req).await
}

async fn website_image(State(website_state): State<WebsiteState>, req: Request) -> Response {
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or_else(|| req.uri().path())
        .to_owned();

    let if_none_match = req
        .headers()
        .get(IF_NONE_MATCH)
        .cloned()
        .map(|header_value| header_value.to_str().unwrap().to_owned());

    let cache_hit = website_state.image_cache.contains_key(&path_and_query);

    let mut response = match website_state
        .image_cache
        .try_get_with(
            path_and_query,
            fetch_image(&website_state.reverse_proxy_config, req),
        )
        .await
    {
        Ok(res) => Box::new(res),
        Err(res) => res.as_ref().to_owned(),
    };

    if cache_hit {
        response
            .headers_mut()
            .insert("x-cache", HeaderValue::from_static("hit"));
    }

    let etag = response
        .headers()
        .get(ETAG)
        .cloned()
        .map(|header_value| header_value.to_str().unwrap().to_owned());

    let not_modified = if_none_match
        .zip(etag)
        .is_some_and(|(if_none_match, etag)| {
            for s in if_none_match.split(',') {
                let candidate = s.trim();
                if candidate == "*" || candidate == etag {
                    return true;
                }
            }

            false
        });

    if not_modified {
        // clean out unnecessary headers for a 304
        let headers = response.headers().clone();
        *response.headers_mut() = HeaderMap::new();

        if let Some(etag_value) = headers.get(ETAG) {
            response.headers_mut().insert(ETAG, etag_value.clone());
        }

        if let Some(etag_value) = headers.get(CACHE_CONTROL) {
            response
                .headers_mut()
                .insert(CACHE_CONTROL, etag_value.clone());
        }

        *response.status_mut() = StatusCode::NOT_MODIFIED;

        *response.body_mut() = Bytes::new();
    }

    response.map(Body::from)
}

async fn fetch_image(
    config: &ReverseProxyConfig,
    req: Request,
) -> Result<Response<Bytes>, Box<Response<Bytes>>> {
    let response = reverse_proxy(config, req)
        .await
        // TODO: we should log this
        .map_err(|_| {
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Bytes::new())
                .unwrap()
        })?;

    let status = response.status();
    let mut headers = response.headers().clone();
    let body = to_bytes(response.into_body(), MAX_IMAGE_BODY)
        .await
        // TODO: this should also be logged
        .map_err(|_| {
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Bytes::new())
                .unwrap()
        })?;

    // hop-by-hop / recomputed for the buffered body
    headers.remove(CONTENT_LENGTH);
    headers.remove(TRANSFER_ENCODING);
    headers.remove(CONNECTION);

    let has_cacheable_header = headers
        .get(CACHE_CONTROL)
        .and_then(|value| value.to_str().ok())
        .filter(|value| value.to_ascii_lowercase().contains("public"))
        .is_some();

    let cacheable = status == StatusCode::OK && has_cacheable_header;

    let mut response = Response::new(body);
    *response.headers_mut() = headers;
    *response.status_mut() = status;

    if cacheable {
        Ok(response)
    } else {
        Err(Box::new(response))
    }
}
