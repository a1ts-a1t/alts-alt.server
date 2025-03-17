use futures_util::future::BoxFuture;
use http_body_util::combinators::BoxBody;
use hyper::body::Bytes;
use hyper::http::{Request, Response};

pub(crate) type RouterRequest = Request<hyper::body::Incoming>;
pub(crate) type RouterResponseBody = BoxBody<Bytes, hyper::Error>;
pub(crate) type RouterResponse = Response<RouterResponseBody>;
pub(crate) type RouterError = hyper::Error;
pub(crate) type RouterFuture = BoxFuture<'static, Result<RouterResponse, RouterError>>;

pub(crate) trait RouteFn: Fn(RouterRequest) -> RouterFuture {
    fn clone_box(&self) -> Box<dyn RouteFn + Send + Sync + 'static>;
}

impl<F> RouteFn for F
where
    F: Fn(RouterRequest) -> RouterFuture + Send + Sync + Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn RouteFn + Send + Sync + 'static> {
		Box::new(self.clone())
    }
}

impl Clone for Box<dyn RouteFn + Send + Sync + 'static> {
    fn clone(&self) -> Self {
		(**self).clone_box()
    }
}

