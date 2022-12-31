use crate::request_builder::{MessageBuilder, MessageRequest, MessageResponse, MessageType};
use log::{error, trace};
use redis::{Client, Commands, ControlFlow, JsonCommands, PubSubCommands, RedisError, RedisResult};
use std::env::VarError;
use std::error::Error;
use std::fmt::format;
use serde::Serialize;
use crate::request_builder;
use crate::structs::{MessageLog, Storable};
use crate::structs::webhooks::Event;

fn create_client() -> Result<Client, RedisError> {
    let url = std::env::var("REDIS_URL").unwrap();
    let client = redis::Client::open(url);


    return match client {
        Ok(client) => unsafe { Ok(client) },
        Err(err) => Err(err),
    };
}

pub fn log_message(message: &MessageLog) -> Result<String, RedisError> {
    let client = create_client()?;
    let mut con = client.get_connection()?;

    let mut systems_list: String = String::from("");

    for system in &message.destination_systems{
        systems_list.push_str(format!("{}", system).as_str())
    }


    let id: RedisResult<String> = con.xadd(
        "whatsapp-logs",
        "*",
        &[
            ("phone_number", &message.phone_number),
            ("origin", &message.origin),
            ("register_id", &message.register_id),
            ("timestamp", &message.timestamp),
            ("timestamp", &systems_list),
        ],
    );

    return match id {
        Ok(id) => Ok(id),
        Err(err) => {
            return Err(err);
        }
    }

}

pub fn publish_message(message: &MessageLog, phone_number: &String) -> Result<String, Box<dyn Error>> {
    let client = create_client()?;
    let mut con = client.get_connection()?;
    let _: () = con
        .publish(
            format!("whatsapp-notification:{}", phone_number),
            serde_json::to_string(message).unwrap(),
        )
        .expect("err");

    Ok("OK".to_string())
}

pub fn create_message(message: &MessageRequest, to: String) -> Result<MessageResponse, Box<dyn Error>> {
    let mut request = MessageBuilder::new();
    let mut responses: Vec<MessageResponse> = vec![];

    return match MessageType::from_str(&message.message_type) {
        MessageType::Text => {
            let request = MessageBuilder::new()
                .message_type(MessageType::Text, None)
                .to(to)
                .body(message.clone().content.body.unwrap())
                .execute();

            match request {
                Ok(response_body) => {
                    Ok(response_body)


                    // let id = &response_body.messages[0].id;;
                    // let log = MessageLog{
                    //     timestamp: "".to_string(),
                    //     destination_systems: vec![0],
                    //     phone_number: receiver.to_string(),
                    //     origin: "OUTGOING".to_string(), //OUTGOING or INCOMING
                    //     register_id: id.to_string(),
                    // };
                    //
                    // // Publish message
                    // publish_message(&log, receiver)?;
                    //
                    // // Store message
                    // store_message(message, receiver, id)?;
                }
                Err(err) => {
                    error!(
                            "{}",
                            format!("Couldnt proccess message creation: {}", err.to_string())
                                .as_str()
                        );
                    Err(err)
                }
            }

        }
        MessageType::InteractiveButton => {

            let mut request = MessageBuilder::new()
                .message_type(
                    MessageType::Interactive,
                    Some(MessageType::InteractiveButton),
                )
                .to(to)
                .header("Pescara Auto".to_string())
                .body(message.clone().content.body.unwrap())
                .clone();

            for button in &message.clone().content.buttons.unwrap().choices {
                request.add_reply_button(button, None);
            }

            let response = request.execute();

            match response {
                Ok(response_body) => {
                    Ok(response_body)
                }
                Err(err) => {
                    error!(
                            "{}",
                            format!("Couldnt proccess message creation: {}", err.to_string())
                                .as_str()
                        );
                    Err(err)
                }
            }

        }

        MessageType::InteractiveList => {
            todo!();
        }
        _ => {
            panic!("Invalid option")
        }
    }

}

// pub fn store_event(event: Event, from: &String, message_id: &String) -> Result<String, RedisError>{
//
//     let client = create_client().unwrap();
//     let mut con = client.get_connection().unwrap();
//
//     let json = serde_json::to_string(&event).unwrap();
//     let key = format!("incoming-messages:{}:{}", from, message_id);
//
//     con.json_set(key,"$", &json)
// }

pub fn store_message(event: &impl Serialize, to: &String, message_id: &String, namespace: &str) -> Result<String, RedisError>{

    let client = create_client().unwrap();
    let mut con = client.get_connection().unwrap();

    let json = serde_json::to_string(&event).unwrap();
    trace!("JSON: {}", json);
    let key = format!("{}:{}:{}", namespace, to, message_id);

    con.json_set(key,"$", &event)?;

    Ok(format!("{}:{}:{}", namespace, to, message_id))
}
