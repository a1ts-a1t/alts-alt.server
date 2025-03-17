use std::collections::HashMap;
use crate::types::{RouterRequest, RouterResponse, RouterError, RouterFuture, RouteFn};
use crate::utils::create_response_body_from_string;
use http::uri::{InvalidUriParts, PathAndQuery};
use hyper::service::Service;
use hyper::{Response, StatusCode};
use itertools::Itertools;

fn route_vec_to_string(route: &Vec<String>) -> String {
    route.iter().map(|component| {
        let mut component_clone = component.clone();
        component_clone.insert(0, '/');
        component_clone
    }).join("")
}

fn trim_route_string_from_request(route_string: &str, req: RouterRequest) -> Result<RouterRequest, InvalidUriParts> {
    let (mut parts, body): (http::request::Parts, _) = req.into_parts();
    let mut uri_parts = parts.uri.into_parts();
    let path_str = uri_parts.path_and_query.as_ref()
        .map(|path_and_query| path_and_query.as_str())
        .map(|s| s.trim_start_matches(route_string))
        .unwrap_or_else(|| panic!("Request has no path"));
    let path_and_query: PathAndQuery = path_str.parse().unwrap_or_else(|e| panic!("Unable to parse path: {}", e));
    uri_parts.path_and_query = Some(path_and_query);
    let new_uri = http::Uri::from_parts(uri_parts);
    match new_uri {
        Ok(new_uri) => {
            parts.uri = new_uri;
            Ok(http::Request::from_parts(parts, body))
        },
        Err(e) => Err(e),
    }
}

#[derive(Clone)]
pub(crate) struct Router {
	route_fn_map: HashMap<Vec<String>, Box<dyn RouteFn + Send + Sync + 'static>>,
}

impl Router {
	pub fn new() -> Router {
		Router {
			route_fn_map: HashMap::new(),
		}
	}

    pub fn with_route_fn<F>(&mut self, route: Vec<String>, func: F) -> ()
    where
        F: Fn(RouterRequest) -> RouterFuture + Clone + Send + Sync + 'static,
    {
        if self.route_fn_map.contains_key(&route) {
            panic!("Router already has a route fn for route {}", route_vec_to_string(&route));
        }

        let route_string = route_vec_to_string(&route);
        self.route_fn_map.insert(route, Box::new(move |req| {
            match trim_route_string_from_request(&route_string, req) {
                Ok(new_req) => {
                    func(new_req)
                },
                Err(e) => panic!("Cannot create URI for route: {}", e),
            }
        }));
    }

    pub fn with_service<S>(&mut self, route: Vec<String>, service: S) -> ()
    where
        S: Service<
            RouterRequest, 
            Response=RouterResponse, 
            Error=RouterError, 
            Future=RouterFuture
        > + Clone + Send + Sync + 'static,
    {
        let func = move |req| service.call(req);
        self.with_route_fn(route, func);
    }
}

impl Service<RouterRequest> for Router {
    type Response = RouterResponse;
    type Error = RouterError;
	type Future = RouterFuture;

    fn call(&self, req: RouterRequest) -> Self::Future {
		let path = req.uri().path();
        let route: Vec<String> = path.trim_matches('/').split('/').map(|s| s.to_string()).collect();

        let func = self.route_fn_map.iter()
            .filter(|kv| route.starts_with(kv.0))
            .sorted_unstable_by_key(|kv| kv.0.len())
            .rev()
            .map(|kv| kv.1)
            .next();

		if func.is_none() {
			let body = create_response_body_from_string(format!("Resource not found for path: {}", path));
			let res = Response::builder()
				.status(StatusCode::NOT_FOUND)
				.body(body)
				.unwrap();
			return Box::pin(async { Ok(res) });
		}

		let func = func.unwrap();
		let fut = func(req);
		Box::pin(async { fut.await })
    }
}

