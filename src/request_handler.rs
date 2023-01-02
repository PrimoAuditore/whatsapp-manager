use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::cookie::time::macros::offset;
use actix_web::cookie::time::OffsetDateTime;
use actix_web::HttpResponse;
use log::{error, trace};
use crate::redis::{create_message, log_message, publish_message, store_message, get_user_mode, get_destination_system, set_last_message, get_user_last_message, get_user_message, set_user_mode};
use crate::request_builder::{MessageContent, MessageRequest, MessageResponse};
use crate::structs::{MessageLog, ModifiedReference, StandardResponse};
use crate::structs::webhooks::Event;

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
                            destination_systems: vec!["0".to_string()],
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


pub fn webhook_message(event: Event) -> Result<StandardResponse, StandardResponse> {
    trace!("{}", serde_json::to_string_pretty(&event).unwrap());

    let mut response: StandardResponse = StandardResponse::new();
    let mut errors:Vec<String> = vec![];
    let mut references = vec![];

    let phone_number = &event.entry[0].changes[0].value.messages.as_ref().unwrap()[0].from.clone();
    let message_id = &event.entry[0].changes[0].value.messages.as_ref().unwrap()[0].id.clone();

    let message_reference = get_user_last_message(&phone_number).unwrap();



    if message_reference != "" {
        // Get user last message linked to previously obtained reference
        let message = get_user_message(message_reference, &phone_number).unwrap();

        // Check expiration time for user last message
        let time_as_integer = &message.entry[0].changes[0].value.messages.as_ref().unwrap()[0].timestamp
            .parse::<i64>()
            .unwrap();


        let parsed_time =
            OffsetDateTime::from_unix_timestamp(*time_as_integer).unwrap().to_offset(offset!(-3));

        let time_difference = SystemTime::now().duration_since(SystemTime::from(parsed_time));

        trace!("since last message: {} secs", time_difference.as_ref().unwrap().as_secs());

        // If message was 6 hours or more ago
        if time_difference.unwrap().as_secs() > 21600 {
            // reset user mode to 0
            set_user_mode(phone_number, "0");
        };

    }


    // Get user active mode
    let mode = get_user_mode(phone_number).unwrap();

    // Get mode destination systems
    let destination_system = get_destination_system(mode);




    // Store json message on redis
    let json_result = store_message(&event.clone(), phone_number, message_id, "incoming-messages");

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
        destination_systems: destination_system.unwrap(),
        phone_number: phone_number.to_string(),
        origin: "INCOMING".to_string(), //OUTGOING or INCOMING
        register_id: message_id.to_string(),
    };

    // Publish notification to channel
    let publish_res = publish_message(&log, phone_number);

    match publish_res {
        Ok(_) => {
            references.push(ModifiedReference { system: "REDIS".to_string(), reference: format!("whatsapp-notification:{}", phone_number)});

            // Store notification sent to channel
            let id = log_message(&log);

            match id {
                Ok(redis_id) => {
                    references.push(ModifiedReference { system: "REDIS".to_string(), reference: redis_id});


                    // Set message id as last message
                    let res = set_last_message(&message_id, &phone_number).unwrap();



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
        Err(response)
    }else{
        Ok(response)
    }


}

pub fn send_menu(log: MessageLog) -> Result<StandardResponse, StandardResponse> {

    let request = MessageRequest{
        system_id: 1,
        to: vec![log.phone_number],
        message_type: "text".to_string(),
        content: MessageContent {
            body: Some("Opciones disponibles:\n 1. Busqueda respuesto.\n 2. Ayuda.".to_string()),
            list: None,
            buttons: None,
        },
    };

    send_message(request)

}