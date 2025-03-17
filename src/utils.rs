use crate::types::RouterResponseBody;
use futures_util::TryStreamExt;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::{Bytes, Frame};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

pub(crate) fn create_response_body_from_string(s: String) -> RouterResponseBody {
    let body = Full::new(Bytes::from(s)).map_err(|e| match e {});
	body.boxed()
}

pub(crate) fn create_response_body_from_file(f: File) -> RouterResponseBody {
    let reader_stream = ReaderStream::new(f);
    let stream_body = StreamBody::new(reader_stream.map_ok(Frame::data).map_err(|e| {
        panic!("Unable to create response body from file: {}", e);
    }));
    stream_body.boxed()
}
