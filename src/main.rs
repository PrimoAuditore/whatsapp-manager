mod structs;
mod redis;

use std::error::Error;
use std::thread;
use std::thread::{sleep, sleep_ms};
use ::redis::RedisError;
use actix_web::{App, get, HttpResponse, HttpServer, post, Responder, web};
use serde_derive::{Serialize, Deserialize};
use crate::redis::log_message;
use crate::structs::{Event, MessageRequest};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let thread = thread::spawn(||{
        redis::subscribe_to_channel();
    });

    sleep_ms(5000);
    print!("{}", thread.is_finished());


    println!("dfsafsdf");
    HttpServer::new(|| {
        App::new()
            .service(webhook)
            .service(send_message)
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}



#[post("/webhook")]
async fn webhook(event: web::Json<Event>) -> impl Responder{
   "OK"
}

#[post("/message")]
async fn send_message(message: web::Json<MessageRequest>) -> impl Responder{
    let created_registers = log_message(message.0);

    return match created_registers {
        Ok(ids) => { HttpResponse::Ok().json(ids) }
        Err(_) => { HttpResponse::InternalServerError().body("Err") }
    }

}