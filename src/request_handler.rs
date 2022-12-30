use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::HttpResponse;
use log::{error, trace};
use crate::redis::{create_message, log_message, publish_message, store_message};
use crate::request_builder::{MessageRequest, MessageResponse};
use crate::structs::{MessageLog, ModifiedReference, StandardResponse};

pub fn send_message(message:MessageRequest) -> Result<StandardResponse, StandardResponse> {

    let mut response: StandardResponse = StandardResponse::new();
    let mut errors = vec![];
    let mut references = vec![];

    // Iterate over receiver
    for receiver in &message.to{

        // Sends the message though whatsapp API
        let created_message = create_message(&message, receiver.to_string());

        match created_message {
            Ok(message_response) => {

                // Add whatsapp id to references
                let id = &message_response.messages[0].id;
                references.push(ModifiedReference{ system: "WHATSAPP".to_string(), reference: id.to_string() });
                trace!("Create message with id: {}", id);


                // Store message
                let store_res = store_message(&message, receiver, id, "outgoing-messages");

                match store_res {
                    Ok(storage_id) => {

                        //Creates log
                        let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
                            Ok(n) => n.as_millis().to_string(),
                            Err(_) => panic!("SystemTime before UNIX EPOCH!"),
                        };

                        let log = MessageLog{
                            timestamp: timestamp,
                            destination_systems: vec![0],
                            phone_number: receiver.to_string(),
                            origin: "OUTGOING".to_string(), //OUTGOING or INCOMING
                            register_id: storage_id.clone(),
                        };


                        // Publish message
                        let publish_res = publish_message(&log, receiver);

                        log_message(&log);

                        references.push(ModifiedReference{ system: "REDIS".to_string(), reference: storage_id.clone().to_string() });
                    }
                    Err(err) => {
                        errors.push(format!("{}", err));
                        error!("{}", err);
                    }
                }


            }
            Err(err) => {
                errors.push(format!("{}", err));
                error!("{}", err);
            }
        }

    }



    response.references = references;

    return if errors.len() > 0 {
        response.errors = Some(errors);
        Err(response)
    }else{
        Ok(response)
    }


}