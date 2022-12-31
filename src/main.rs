extern crate core;

mod error_manager;
mod redis;
mod request_builder;
mod requests;
mod structs;
mod request_handler;

use crate::error_manager::get_public_error;
use crate::redis::{create_message, log_message, publish_message, store_message};
use crate::request_builder::{MessageContent, MessageRequest, MessageResponse};
use crate::structs::webhooks::Event;
use crate::structs::{MessageLog, ModifiedReference, StandardResponse};
use ::redis::RedisError;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, };
use log::{error, trace};
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
            .service(send_message)
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
    trace!("{}", serde_json::to_string_pretty(&event.0).unwrap());

    let mut response: StandardResponse = StandardResponse::new();
    let mut errors:Vec<String> = vec![];
    let mut references = vec![];

    let phone_number = &event.entry[0].changes[0].value.messages.as_ref().unwrap()[0].from.clone();
    let message_id = &event.entry[0].changes[0].value.messages.as_ref().unwrap()[0].id.clone();


    // Store json message on redis
    let json_result = redis::store_message(&event.0.clone(), phone_number, message_id, "incoming-messages");

    match json_result {
        Ok(_) => {
            references.push(ModifiedReference { system: "REDIS".to_string(), reference: format!("incoming-messages:{}:{}", phone_number, message_id) })
        }
        Err(err) => {
            errors.push(format!("{}", err))
        }
    }



    // Build notification log
    let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => n.as_millis().to_string(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    };

    let log = MessageLog{
        timestamp: timestamp,
        destination_systems: vec![0],
        phone_number: phone_number.to_string(),
        origin: "INCOMING".to_string(), //OUTGOING or INCOMING
        register_id: message_id.to_string(),
    };

    // Publish notification to channel
    let publish_res = publish_message(&log, phone_number);

    match publish_res {
        Ok(_) => {
            references.push(ModifiedReference { system: "REDIS".to_string(), reference: format!("whatsapp-notification:{}", phone_number)});

            // Only log is message was published succesfully

            // Store notification sent to channel
            let id = log_message(&log);

            match id {
                Ok(redis_id) => {
                    references.push(ModifiedReference { system: "REDIS".to_string(), reference: redis_id});

                }
                Err(err) => {
                    errors.push(format!("{}", err))
                }
            }

        }
        Err(err) => {
            errors.push(format!("{}", err))
        }
    };

    // Build response
    response.references = references;

    if errors.len() > 0 {
        response.errors = Some(errors);
    }

    return if response.errors.is_some() {
        HttpResponse::InternalServerError().body(serde_json::to_string(&response).unwrap())
    } else {
        HttpResponse::Ok().body(serde_json::to_string(&response).unwrap())
    };

}

#[post("/message")]
async fn send_message(message: web::Json<MessageRequest>) -> impl Responder {
    let response = request_handler::send_message(message.0);

    match response {
        Ok(response) => HttpResponse::Ok().body(serde_json::to_string(&response).unwrap()),
        Err(response) => HttpResponse::InternalServerError().body(serde_json::to_string(&response).unwrap())
    }
}
