use crate::types::{RouterError, RouterFuture, RouterRequest, RouterResponse};
use crate::utils::{create_response_body_from_file, create_response_body_from_string};
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use http::{Response, StatusCode};
use hyper::service::Service;
use std::path::{Path, PathBuf};
use tokio::fs::File;

fn path_to_mime_type(path: &PathBuf) -> Option<String> {
    let extension = path.extension()
        .map(|os_str| os_str.to_str())
        .unwrap_or(None)
        .unwrap_or("");

    match extension {
        "html" => Some("text/html".to_string()),
        "css" => Some("text/css".to_string()),
        "js" => Some("text/javascript".to_string()),
        "ico" => Some("image/vnd.microsoft.icon".to_string()),
        "jpeg" => Some("image/jpeg".to_string()),
        "jpg" => Some("image/jpeg".to_string()),
        "png" => Some("image/png".to_string()),
        "svg" => Some("image/svg+xml".to_string()),
        _ => None,
    }
}

#[derive(Clone)]
pub struct StaticServer {
	root: Option<PathBuf>,
	fallback_file: Option<PathBuf>,
}

impl StaticServer {
	pub fn new() -> StaticServer {
		StaticServer {
			root: None,
			fallback_file: None,
		}
	}

	pub fn with_root(&mut self, root: String) -> () {
		let path = Path::new(&root);
		if !path.is_dir() {
			panic!("Unable to use `{}` as a static server root since it is not a directory", root);
		}
		
		if path.is_absolute() {
			eprintln!("`{}` is an absolute path. This is generally unintended and has severe security implications", &root);
		}

		match Path::canonicalize(path) {
			Ok(canon_path) => self.root = Some(canon_path),
			Err(e) => panic!("Unable to use `{}` as a static server root: {}", root, e),
		};
	}

	pub fn with_fallback_file(&mut self, fallback_file_path: String) -> () {
		let path = Path::new(&fallback_file_path);
		if path.is_dir() {
			panic!("Unable to use `{}` as a fallback file since it is not a directory", fallback_file_path);
		}
		
		if path.is_absolute() {
			eprintln!("`{}` is an absolute path. This is generally unintended and has severe security implications", &fallback_file_path);
		}

		match Path::canonicalize(path) {
			Ok(canon_path) => self.fallback_file = Some(canon_path),
			Err(e) => panic!("Unable to use `{}` as a fallback file: {}", fallback_file_path, e),
		};
	}

	fn get_not_found_response(&self, uri_path: String) -> RouterFuture {
		let mapped_fallback_file: BoxFuture<Option<File>> = match &self.fallback_file {
			None => Box::pin(async { None }),
			Some(fallback_file) => {
                let fallback_file_clone = fallback_file.clone();
                Box::pin(async move { File::open(fallback_file_clone)
                    .map(|res| match res {
                        Err(_) => None ,
                        Ok(file) => Some(file),
                    }).await })
            }
		};

        let fut = mapped_fallback_file.map(move |opt_file| {
            match opt_file {
                None => {
                    let body = create_response_body_from_string(format!("Resource not found: {}", uri_path));
                    Ok(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(body)
                        .unwrap())
                },
                Some(file) => {
                    let body = create_response_body_from_file(file);
                    Ok(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(body)
                        .unwrap())
                },
            }
        });

        Box::pin(async { fut.await })
	}
}

impl Service<RouterRequest> for StaticServer {
    type Response = RouterResponse;
    type Error = RouterError;
	type Future = RouterFuture;

    fn call(&self, req: RouterRequest) -> Self::Future {
        if self.root.is_none() {
            panic!("Cannot invoke static server without a root path");
        }

        let root_path = self.root.clone().unwrap();
        let uri_path = req.uri().path().trim_start_matches("/");
        let not_found_response = self.get_not_found_response(uri_path.to_string());
        let file_path = root_path.join(Path::new(uri_path));
        let mime_type = path_to_mime_type(&file_path);
        let file_future = Path::canonicalize(&file_path).iter()
            .map(|path| path.clone())
            .filter(|path| path.starts_with(&root_path))
            .filter(|path| !path.is_dir())
            .map(|path| File::open(path))
            .next();

        if file_future.is_none() {
            return Box::pin(async { not_found_response.await });
        }
        
        let fut = file_future.unwrap().then(|file_res| {
            match file_res {
                Err(e) => {
                    eprintln!("Unable to create file from path: {}", e);
                    not_found_response
                },
                Ok(file) => {
                    Box::pin(async { 
                        let res = match mime_type {
                            Some(response_mime_type) => Response::builder()
                                .header("Content-Type", response_mime_type)
                                .status(StatusCode::OK)
                                .body(create_response_body_from_file(file))
                                .unwrap(),
                            None => Response::builder()
                                .status(StatusCode::OK)
                                .body(create_response_body_from_file(file))
                                .unwrap(),
                        };
                        Ok(res)
                    })
                },
            }
        });

        Box::pin(async { fut.await })
    }
}

