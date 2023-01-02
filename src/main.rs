extern crate core;

mod error_manager;
mod redis;
mod request_builder;
mod requests;
mod structs;
mod request_handler;

use std::collections::HashMap;
use std::env;
use crate::error_manager::get_public_error;
use crate::redis::{create_message, log_message, publish_message, store_message};
use crate::request_builder::{MessageContent, MessageRequest, MessageResponse};
use crate::structs::webhooks::Event;
use crate::structs::{MessageLog, ModifiedReference, StandardResponse};
use ::redis::RedisError;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, HttpRequest};
use log::{debug, error, trace};
use serde_derive::{Deserialize, Serialize};
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::middleware::Logger;


static SYSTEM_ID: &str = "01";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::new("%U"))
            .service(health)
            .service(webhook)
            .service(validate)
            .service(send_message)
            .service(send_menu)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

#[get("/health")]
async fn health() -> impl Responder {
    "OK"
}

#[post("/webhook")]
async fn webhook(event: web::Json<Event>) -> impl Responder {
    let response = request_handler::webhook_message(event.0);

    match response {
        Ok(response) => HttpResponse::Ok().body(serde_json::to_string(&response).unwrap()),
        Err(response) => HttpResponse::InternalServerError().body(serde_json::to_string(&response).unwrap())
    }

}

#[post("/send-menu")]
async fn send_menu(log: web::Json<MessageLog>) -> impl Responder {
    let response = request_handler::send_menu(log.0);

    match response {
        Ok(response) => HttpResponse::Ok().body(serde_json::to_string(&response).unwrap()),
        Err(response) => HttpResponse::InternalServerError().body(serde_json::to_string(&response).unwrap())
    }

}


#[get("/webhook")]
async fn validate(validation_parameters: HttpRequest) -> impl Responder {
    let verify_token = match env::var("VERIFY_TOKEN") {
        Ok(x) => x,
        Err(err) => panic!("{}", err),
    };


    let mut param_map = HashMap::new();

    for param in validation_parameters.query_string().split("&") {
        let param_vec: Vec<&str> = param.split("=").collect();
        param_map.insert(param_vec[0], param_vec[1]);
    }

    if verify_token != param_map.get("hub.verify_token").unwrap().to_string() {
        panic!("Received verification token is not equals to defined one")
    }

    debug!("{:?}", &param_map);

    HttpResponse::Ok().body(param_map.get("hub.challenge").unwrap().to_string())
}


#[post("/message")]
async fn send_message(message: web::Json<MessageRequest>) -> impl Responder {
    let response = request_handler::send_message(message.0);

    match response {
        Ok(response) => HttpResponse::Ok().body(serde_json::to_string(&response).unwrap()),
        Err(response) => HttpResponse::InternalServerError().body(serde_json::to_string(&response).unwrap())
    }
}
