extern crate hyper;
extern crate futures;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate serde_json;

use std::{ collections::HashMap, io };

use hyper::{
    header::{ ContentLength, ContentType },
    server::{ Request, Response, Service },
    Chunk,
    Method,
    StatusCode,
};
use futures::{ future::{ Future, FutureResult }, Stream };

struct MicroService;

impl Service for MicroService {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<dyn Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        info!("Microservice received a request {:?}", req);
        match (&req.method(), req.path()) {
            (&Method::Post, "/") => {
                let future = req
                    .body()
                    .concat2()
                    .and_then(parse_form)
                    .and_then(write_to_db)
                    .then(make_post_response);
                Box::new(future)
            }
            _ => Box::new(futures::future::ok(Response::new().with_status(StatusCode::NotFound))),
        }
    }
}

struct NewMessage {
    username: String,
    message: String,
}

fn main() {
    env_logger::init();
    let address = "127.0.0.1:8080".parse().unwrap();
    let server = hyper::server::Http
        ::new()
        .bind(&address, || Ok(MicroService {}))
        .unwrap();

    info!("Running microservice at {}", address);
    server.run().unwrap();
}

fn parse_form(form_chunk: Chunk) -> FutureResult<NewMessage, hyper::Error> {
    let mut form = url::form_urlencoded
        ::parse(form_chunk.as_ref())
        .into_owned()
        .collect::<HashMap<String, String>>();

    if let Some(message) = form.remove("message") {
        let username = form.remove("username").unwrap_or(String::from("Anon"));
        futures::future::ok(NewMessage {
            username,
            message,
        })
    } else {
        futures::future::err(
            hyper::Error::from(
                io::Error::new(io::ErrorKind::InvalidInput, "Missing field `message`")
            )
        )
    }
}

fn write_to_db(entry: NewMessage) -> FutureResult<i64, hyper::Error> {
    futures::future::ok(0)
}

fn make_post_response(
    result: Result<i64, hyper::Error>
) -> FutureResult<hyper::Response, hyper::Error> {
    match result {
        Ok(timestamp) => {
            let payload = serde_json::json!({"timestamp:": timestamp}).to_string();
            debug!("{:?}", &payload);
            let response = Response::new()
                .with_header(ContentLength(payload.len() as u64))
                .with_header(ContentType::json())
                .with_body(payload);
            futures::future::ok(response)
        }
        Err(error) => make_error_response(&error.to_string(), StatusCode::BadRequest),
    }
}

fn make_error_response(
    error_message: &str,
    status_code: StatusCode
) -> FutureResult<hyper::Response, hyper::Error> {
    let payload = serde_json::json!({"error": error_message}).to_string();
    debug!("{:?}", &payload);
    let response = Response::new()
        .with_status(status_code)
        .with_header(ContentLength(payload.len() as u64))
        .with_header(ContentType::json())
        .with_body(payload);
    futures::future::ok(response)
}
