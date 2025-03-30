use crate::types::{RouteFn, RouterError, RouterFuture, RouterRequest, RouterResponse};
use futures_util::FutureExt;
use hyper::http::Response;
use http_body_util::combinators::BoxBody;
use hyper::{body::Bytes, service::Service};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{Duration, SystemTime};

#[derive(Clone)]
struct CacheService {
    max_age: Duration,
	service_fn: Box<dyn RouteFn + Send + Sync + 'static>,

	cached_data: Arc<Mutex<Option<Response<BoxBody<Bytes, hyper::Error>>>>>,
	refresh_time: Arc<Mutex<Option<SystemTime>>>,
}

impl CacheService {
    pub fn from_route_fn<F>(func: F, max_age: Duration) -> CacheService
    where
        F: Fn(RouterRequest) -> RouterFuture + Clone + Send + Sync + 'static,
    {
		CacheService {
			max_age,
			service_fn: Box::new(move |req| func(req)),
			cached_data: Arc::new(Mutex::new(None)),
			refresh_time: Arc::new(Mutex::new(None)),
		}
    }
}

impl Service<RouterRequest> for CacheService {
    type Response = RouterResponse;
    type Error = RouterError;
    type Future = RouterFuture;

    fn call(&self, req: RouterRequest) -> Self::Future {
        let func = async || {
            let service_fn = self.service_fn.clone();
            let max_age = self.max_age.clone();

            let mut cached_data_mutex = self.cached_data.lock().await;
            let mut refresh_time_mutex = self.refresh_time.lock().await;

            let is_cache_fresh = (*refresh_time_mutex)
                .map(|refresh_time| SystemTime::now().duration_since(refresh_time))
                .iter()
                .flat_map(|result| match result {
                    Ok(duration) => Some(duration),
                    Err(_) => None,
                })
                .map(|duration| duration.le(&max_age))
                .next()
                .unwrap_or(false);

            let fresh_data = (*cached_data_mutex).filter(|_| is_cache_fresh);

            if let Some(response) = fresh_data {
                return Ok(response);
            }

            let res = (*service_fn)(req).await;
            match res {
                Ok(response) => {
                    *cached_data_mutex = Some(response);
                    *refresh_time_mutex = Some(SystemTime::now());
                    return Ok(response);
                },
                Err(e) => Err(e),
            }
        };

        Box::pin(async { func().await })
    }
}

