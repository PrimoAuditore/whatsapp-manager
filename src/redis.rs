use crate::request_builder;
use crate::request_builder::{MessageBuilder, MessageRequest, MessageResponse, MessageType};
use crate::structs::webhooks::Event;
use crate::structs::{MessageLog, Storable};
use log::{debug, error, trace};
use redis::{Client, Commands, ControlFlow, JsonCommands, PubSubCommands, RedisError, RedisResult};
use serde::Serialize;
use std::env::VarError;
use std::error::Error;

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

    for system in &message.destination_systems {
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
    };
}

pub fn publish_message(
    message: &MessageLog,
    phone_number: &String,
) -> Result<String, Box<dyn Error>> {
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

pub fn create_message(
    message: &MessageRequest,
    to: String,
) -> Result<MessageResponse, Box<dyn Error>> {
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
                Ok(response_body) => Ok(response_body),
                Err(err) => {
                    error!(
                        "{}",
                        format!("Couldnt proccess message creation: {}", err.to_string()).as_str()
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
                Ok(response_body) => Ok(response_body),
                Err(err) => {
                    error!(
                        "{}",
                        format!("Couldnt proccess message creation: {}", err.to_string()).as_str()
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
    };
}

pub fn get_user_mode(phone_number: &str) -> Result<u16, RedisError> {
    let client = create_client().unwrap();
    let mut con = client.get_connection().unwrap();

    let mode: RedisResult<String> = con.hget(format!("selected-mode:{}", phone_number), "mode");

    if mode.is_err() {
        let is_nil = is_nil(mode.as_ref().unwrap_err());
        // Sets user mode to 0 in case its the first message
        return if is_nil {
            set_user_mode(phone_number, "100");
            Ok(0)
        } else {
            Err(mode.unwrap_err())
        };
    }

    let parsed_mode = mode.unwrap().parse::<u16>().unwrap();

    Ok(parsed_mode)
}

pub fn set_user_mode(phone_number: &str, mode: &str) -> Result<String, RedisError> {
    let client = create_client().unwrap();
    let mut con = client.get_connection().unwrap();

    let mode: RedisResult<String> =
        con.hset(format!("selected-mode:{}", phone_number), "mode", mode);

    if mode.is_ok() {
        Ok(mode.unwrap().clone())
    } else {
        Err(mode.unwrap_err())
    }
}

pub fn store_message(
    event: &impl Serialize,
    to: &String,
    message_id: &String,
    namespace: &str,
) -> Result<String, RedisError> {
    let client = create_client().unwrap();
    let mut con = client.get_connection().unwrap();

    let json = serde_json::to_string(&event).unwrap();
    trace!("JSON: {}", json);
    let key = format!("{}:{}:{}", namespace, to, message_id);

    con.json_set(key, "$", &event)?;

    Ok(format!("{}:{}:{}", namespace, to, message_id))
}

pub fn get_destination_system(mode: u16) -> Result<Vec<String>, RedisError> {
    let client = create_client().unwrap();
    let mut con = client.get_connection().unwrap();

    let mode_list = con.lrange(format!("mode-systems:{}", mode), 0, 100);

    if mode_list.is_err() {
        if is_nil(&mode_list.as_ref().unwrap_err()) {
            return Ok(vec![]);
        }

        return Err(mode_list.unwrap_err());
    }

    Ok(mode_list.unwrap())
}

pub fn set_last_message(id: &str, phone_number: &str) -> Result<String, RedisError> {
    let client = create_client().unwrap();
    let mut con = client.get_connection().unwrap();

    let res: String = con
        .set(format!("last-message:{}", phone_number), id)
        .unwrap();

    Ok(res)
}

pub fn get_user_last_message(phone_number: &str) -> Result<String, RedisError> {
    let client = create_client().unwrap();
    let mut con = client.get_connection().unwrap();

    let mut res: RedisResult<String> = con.get(format!("last-message:{}", phone_number));

    if res.is_err() {
        let is_nil = is_nil(&res.as_ref().unwrap_err());

        // Check if it is phone numbers first message
        if is_nil {
            set_last_message("", phone_number);

            return Ok("".to_string());
        }
        return Err(res.unwrap_err());
    }

    Ok(res.unwrap())
}

pub fn get_user_message(message_id: String, phone_number: &str) -> Result<Event, RedisError> {
    let client = create_client().unwrap();
    let mut con = client.get_connection().unwrap();

    let res: String = con
        .json_get(
            format!("incoming-messages:{}:{}", phone_number, message_id),
            ".",
        )
        .unwrap();

    let event: Event = serde_json::from_str(&res).unwrap();

    Ok(event)
}

pub fn is_nil(error: &RedisError) -> bool {
    return if error.to_string().contains("response was nil") {
        true
    } else {
        false
    };
}
