use std::env::VarError;
use std::error::Error;
use redis::{Client, RedisResult, RedisError, Commands};
use crate::structs::MessageRequest;


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

pub fn log_message(message: MessageRequest) -> Result<Vec<String>, RedisError>{
    let client = create_client()?;
    let mut con = client.get_connection()?;
    let mut created_registers: Vec<String> = vec![];

    for receiver in &message.to {


        let id: RedisResult<String> = con.xadd(format!("whatsapp-messages:{}", receiver), "*", &[
            ("from", receiver),
            ("body", &message.body),
            ("system-id", &format!("{}", &message.system_id)),
        ]);

        publish_message(&message);

        match id {
            Ok(id) => { created_registers.push(id) }
            Err(err) => { return Err(err) }
        }

    }

    Ok(created_registers)
}

pub fn subscribe_to_channel() -> Result<String, Box<dyn Error>> {
    let client = create_client()?;
    let mut con = client.get_connection()?;
    let mut pubsub = con.as_pubsub();
    pubsub.subscribe("whatsapp-notification:*")?;

    loop {
        println!("loop");
        let msg = pubsub.get_message()?;
        let payload : String = msg.get_payload()?;
        println!("channel '{}': {}", msg.get_channel_name(), payload);
    }
}

fn publish_message(message: &MessageRequest)-> Result<String, Box<dyn Error>>{
    let client = create_client()?;
    let mut con = client.get_connection()?;
    let _: () = con.publish(format!("whatsapp-notification:{}", message.to[0]), &message.body).expect("err");

    Ok("".to_string())

}