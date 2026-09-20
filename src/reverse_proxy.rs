use axum::{body::Body, extract::{Request, State}, http::Uri, response::{IntoResponse, Response}};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use hyper::StatusCode;

type Client = hyper_util::client::legacy::Client<HttpConnector, Body>;

#[derive(Clone)]
pub struct ReverseProxyConfig {
    client: Client,
    address: String,
}

impl ReverseProxyConfig {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            client: hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
                .build(HttpConnector::new()),
            address: address.into(),
        }
    }
}

pub async fn reverse_proxy(State(config): State<ReverseProxyConfig>, mut req: Request) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);

    let uri = format!("{}{path_and_query}", config.address);

    *req.uri_mut() = Uri::try_from(uri).unwrap();

    Ok(config.client
        .request(req)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .into_response())
}

