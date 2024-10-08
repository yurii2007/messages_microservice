extern crate hyper;
extern crate futures;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate serde_json;

use crate::models::{ Message, NewMessage };
use maud::html;
use std::env;
use std::{ collections::HashMap, error::Error, io };
use hyper::{
    header::{ ContentLength, ContentType },
    server::{ Request, Response, Service },
    Chunk,
    Method::{ self, Get },
    StatusCode,
};
use futures::{ future::{ Future, FutureResult }, Stream };
use diesel::prelude::*;
use diesel::pg::PgConnection;

extern crate serde_derive;
extern crate diesel;
mod models;
mod schema;

const DEFAULT_DATABASE_URL: &'static str = "postgresql://postgres@localhost:5432";

struct MicroService;

impl Service for MicroService {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<dyn Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let mut db_connection = match connect_to_db() {
            Some(connection) => connection,
            None => {
                return Box::new(
                    futures::future::ok(
                        Response::new().with_status(StatusCode::InternalServerError)
                    )
                );
            }
        };

        info!("Microservice received a request {:?}", req);
        match (&req.method(), req.path()) {
            (&Method::Post, "/") => {
                let future = req
                    .body()
                    .concat2()
                    .and_then(parse_form)
                    .and_then(move |new_message| write_to_db(new_message, &mut db_connection))
                    .then(make_post_response);
                Box::new(future)
            }
            (&Get, "/") => {
                let time_range = match req.query() {
                    Some(query) => parse_query(query),
                    None =>
                        Ok(TimeRange {
                            before: None,
                            after: None,
                        }),
                };
                let response = match time_range {
                    Ok(time_range) => make_get_response(query_db(time_range, &mut db_connection)),
                    Err(error) => make_error_response(&error.to_string(), StatusCode::NotFound),
                };
                Box::new(response)
            }
            _ => Box::new(futures::future::ok(Response::new().with_status(StatusCode::NotFound))),
        }
    }
}

struct TimeRange {
    before: Option<i64>,
    after: Option<i64>,
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

fn write_to_db(
    new_message: NewMessage,
    db_connection: &mut PgConnection
) -> FutureResult<i64, hyper::Error> {
    use schema::messages;
    let message = diesel
        ::insert_into(messages::table)
        .values(&new_message)
        .returning(Message::as_returning())
        .get_result(db_connection);

    match message {
        Ok(message) => futures::future::ok(message.timestamp),
        Err(error) => {
            error!("Error writing to the database, {}", error.to_string());
            futures::future::err(
                hyper::Error::from(io::Error::new(io::ErrorKind::Other, "service error"))
            )
        }
    }
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

fn make_get_response(
    messages: Option<Vec<Message>>
) -> FutureResult<hyper::Response, hyper::Error> {
    let response = match messages {
        Some(messages) => {
            let body = render_page(messages);
            Response::new()
                .with_header(ContentLength(body.len() as u64))
                .with_body(body)
        }
        None => Response::new().with_status(StatusCode::InternalServerError),
    };

    debug!("{:?}", response);
    futures::future::ok(response)
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

fn parse_query(query: &str) -> Result<TimeRange, String> {
    let args = url::form_urlencoded
        ::parse(&query.as_bytes())
        .into_owned()
        .collect::<HashMap<String, String>>();

    let before = args.get("before").map(|value| value.parse::<i64>());
    if let Some(ref result) = before {
        if let Err(ref error) = *result {
            return Err(format!("Error parsing `before` value: {}", error));
        }
    }

    let after = args.get("after").map(|value| value.parse::<i64>());
    if let Some(ref result) = after {
        if let Err(ref error) = *result {
            return Err(format!("Error parsing `after` value: {}", error));
        }
    }

    Ok(TimeRange { before: before.map(|b| b.unwrap()), after: after.map(|a| a.unwrap()) })
}

fn render_page(messages: Vec<Message>) -> String {
    let markup = html! {
        body {
            style {
                "body { width: 100vw; height: 100vh; background-color: #1D1D1D; color: #FFFCFA}"
            }
            ul {
                @for message in &messages {
                    li {
                        (message.username) " (" (message.timestamp) "): " (message.message)
                    }
                }
            }
        }
    };

    markup.into_string()
}

fn query_db(time_range: TimeRange, connection: &mut PgConnection) -> Option<Vec<Message>> {
    use schema::messages;
    let TimeRange { before, after } = time_range;
    let query_result = match (before, after) {
        (Some(before), Some(after)) => {
            messages::table
                ::select(messages::table, messages::all_columns)
                .get_results::<Message>(connection)
        }
        _ =>
            messages::table
                ::select(messages::table, messages::all_columns)
                .get_results::<Message>(connection),
    };

    match query_result {
        Ok(result) => Some(result),
        Err(error) => {
            error!("Error querying db: {}", error);
            None
        }
    }
}

fn connect_to_db() -> Option<PgConnection> {
    let database_url = env::var("DATABASE_URL").unwrap_or(String::from(DEFAULT_DATABASE_URL));
    match PgConnection::establish(&database_url) {
        Ok(connection) => Some(connection),
        Err(error) => {
            error!("Error connecting to database: {}", error.to_string());
            None
        }
    }
}
