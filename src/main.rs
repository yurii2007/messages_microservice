extern crate hyper;
extern crate futures;

#[macro_use]
extern crate log;
extern crate env_logger;

use hyper::server::{ Request, Response, Service };
use futures::future::Future;

struct MicroService;

impl Service for MicroService {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<dyn Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        info!("Microservice received a request {:?}", req);
        Box::new(futures::future::ok(Response::new().with_body("hello world")))
    }
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
