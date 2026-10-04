use axum::{
    body::Body,
    extract::{Request, State},
    http::{Uri, header},
    response::{IntoResponse, Response},
};
use hyper::StatusCode;
use hyper_tls::HttpsConnector;
use hyper_util::client::legacy::Client as HyperClient;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;

type Client = HyperClient<HttpsConnector<HttpConnector>, Body>;

#[derive(Clone)]
pub struct ReverseProxyConfig {
    client: Client,
    address: String,
}

impl ReverseProxyConfig {
    pub fn new(address: impl Into<String>) -> Self {
        let https = HttpsConnector::new();
        let executor = TokioExecutor::new();
        let client = HyperClient::<(), ()>::builder(executor).build(https);

        Self {
            client,
            address: address.into(),
        }
    }
}

pub async fn reverse_proxy(
    State(config): State<ReverseProxyConfig>,
    mut req: Request,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);

    let uri = Uri::try_from(format!("{}{path_and_query}", config.address))
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    // override host header
    if let Some(authority) = uri.authority() {
        let host = authority
            .as_str()
            .parse()
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
        req.headers_mut().insert(header::HOST, host);
    }

    *req.uri_mut() = uri;

    Ok(config
        .client
        .request(req)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .into_response())
}
