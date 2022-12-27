use std::env::VarError;
use std::error::Error;
use log::error;
use redis::{Client, RedisResult, RedisError, Commands, PubSubCommands, ControlFlow};
use crate::request_builder::{MessageBuilder, MessageRequest, MessageResponse, MessageType};


fn create_client() -> Result<Client, RedisError> {
    let url = std::env::var("REDIS_URL").unwrap();
    let client = redis::Client::open(url);

    return match client {
        Ok(client) => unsafe {
            Ok(client)
        }
        Err(err) => {
            Err(err)
        }
    };
}

pub fn log_message(message: MessageRequest) -> Result<Vec<String>, RedisError> {
    let client = create_client()?;
    let mut con = client.get_connection()?;
    let mut created_registers: Vec<String> = vec![];

    for receiver in &message.to {
        let id: RedisResult<String> = con.xadd(format!("whatsapp-messages:{}", receiver), "*", &[
            ("from", receiver),
            ("body", &message.clone().content.body.unwrap()),
            ("system-id", &format!("{}", &message.system_id)),
        ]);

        publish_message(&message).expect("Error publishing message");

        match id {
            Ok(id) => { created_registers.push(id) }
            Err(err) => {
                return Err(err);
            }
        }
    }

    Ok(created_registers)
}

fn publish_message(message: &MessageRequest) -> Result<String, Box<dyn Error>> {
    let client = create_client()?;
    let mut con = client.get_connection()?;
    let _: () = con.publish(format!("whatsapp-notification:{}", message.to[0]), &message.clone().content.body.unwrap()).expect("err");

    Ok("".to_string())
}

pub fn create_message(message: &MessageRequest) -> Result<Vec<MessageResponse>, Box<dyn Error>> {
    let mut request = MessageBuilder::new();
    let mut responses: Vec<MessageResponse> = vec![];


    match MessageType::from_str(&message.message_type) {
        MessageType::Text => {
            for receiver in &message.to {
                let request  = MessageBuilder::new()
                    .message_type(MessageType::Text, None)
                    .to(receiver.as_str().to_string())
                    .body(message.clone().content.body.unwrap())
                    .execute();

                match request {
                    Ok(response_body) => {
                        responses.push(response_body);
                    }
                    Err(err) => {
                        error!("{}", format!("Couldnt proccess message creation: {}", err.to_string()).as_str());
                        panic!("{}", format!("Couldnt proccess message creation: {}", err.to_string()).as_str());
                    }
                }
            }
        }
        MessageType::InteractiveButton => {


            for receiver in &message.to {
                let mut request = MessageBuilder::new()
                    .message_type(MessageType::Interactive, Some(MessageType::InteractiveButton))
                    .to(receiver.as_str().to_string())
                    .header("Pescara Auto".to_string())
                    .body(message.clone().content.body.unwrap())
                    .clone();

                for button in &message.clone().content.buttons.unwrap().choices {
                    request.add_reply_button(button, None);
                }

                let response = request.execute();


                match response {
                    Ok(response_body) => {
                        responses.push(response_body);
                    }
                    Err(err) => {
                        error!("{}", format!("Couldnt proccess message creation: {}", err.to_string()).as_str());
                        panic!("{}", format!("Couldnt proccess message creation: {}", err.to_string()).as_str());
                    }
                }
            }

            let mut clone = request.clone();
        }

        MessageType::InteractiveList => {
            todo!();
        }
        _ => {}
    }

    Ok(responses)
}

