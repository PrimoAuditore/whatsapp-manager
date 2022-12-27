mod structs;
mod redis;
mod error_manager;
mod requests;
mod request_builder;

use std::error::Error;
use std::thread;
use std::thread::{sleep, sleep_ms};
use ::redis::RedisError;
use actix_web::{App, get, HttpResponse, HttpServer, post, Responder, web};
use log::error;
use serde_derive::{Serialize, Deserialize};
use crate::error_manager::get_public_error;
use crate::redis::{create_message, log_message};
use crate::request_builder::{MessageRequest, MessageResponse};
use crate::structs::{ModifiedReference, StandardResponse};
use crate::structs::webhooks::Event;

static SYSTEM_ID: &str = "01";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    HttpServer::new(|| {
        App::new()
            .service(health)
            .service(webhook)
            .service(send_message)
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}



#[get("/health")]
async fn health() -> impl Responder{
    "OK"
}

#[post("/webhook")]
async fn webhook(event: web::Json<Event>) -> impl Responder{
   "OK"
}


#[post("/message")]
async fn send_message(message: web::Json<MessageRequest>) -> impl Responder{

    let mut response: StandardResponse = StandardResponse::new();
    let mut errors = vec![];
    let mut references = vec![];
    let created_message = create_message(&message);


    match created_message {
        Ok(responses) => {

            for mut response in &responses{
                let id = &response.messages[0].id;
                references.push(ModifiedReference{ system: "WHATSAPP".to_string(), reference: id.to_string()})
            }
        }
        Err(err) => {

            errors.push(err.to_string());

            error!("{}", format!("Message could not be sent"));
            panic!("{}",format!("Message could not be sent"));
        }
    }


    let created_registers = log_message(message.0);

    match created_registers {
        Ok(redis_id) => {

            for id in redis_id{
                references.push(ModifiedReference{ system: "REDIS".to_string(), reference: id })
            }

        }
        Err(err) => {
            errors.push(err.to_string());
        }
    }

    response.references = references;

    if errors.len() > 0 {
        response.errors = Some(errors);
    }

    return if response.errors.is_some() {
        HttpResponse::InternalServerError().body(serde_json::to_string(&response).unwrap())
    }else{
        HttpResponse::Ok().body(serde_json::to_string(&response).unwrap())
    }
}

