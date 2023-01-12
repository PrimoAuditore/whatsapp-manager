use crate::redis::{
    create_message, get_destination_system, get_user_last_message, get_user_message, get_user_mode,
    log_message, publish_message, set_last_message, set_user_mode, store_message,
};
use crate::request_builder::{MessageContent, MessageRequest, MessageResponse};
use crate::structs::webhooks::Event;
use crate::structs::{MessageLog, ModifiedReference, StandardResponse};
use actix_web::cookie::time::macros::offset;
use actix_web::cookie::time::OffsetDateTime;
use actix_web::HttpResponse;
use log::{debug, error, info, trace};
use redis::RedisError;
use serde::de::Unexpected::Str;
use std::error::Error;
use std::fmt::format;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn send_message(message: MessageRequest) -> Result<StandardResponse, StandardResponse> {
    let mut response: StandardResponse = StandardResponse::new();
    let mut errors = vec![];
    let mut references = vec![];

    // Iterate over receiver
    for receiver in &message.to {
        // Sends the message though whatsapp API
        let created_message = create_message(&message, receiver.to_string());

        match created_message {
            Ok(message_response) => {
                // Add whatsapp id to references
                let id = &message_response.messages[0].id;
                references.push(ModifiedReference {
                    system: "WHATSAPP".to_string(),
                    reference: id.to_string(),
                });
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

                        let log = MessageLog {
                            timestamp: timestamp,
                            destination_systems: vec!["0".to_string()],
                            phone_number: receiver.to_string(),
                            origin: "OUTGOING".to_string(), //OUTGOING or INCOMING
                            register_id: storage_id.clone(),
                        };

                        // Publish message
                        let publish_res = publish_message(&log, receiver);

                        log_message(&log);

                        references.push(ModifiedReference {
                            system: "REDIS".to_string(),
                            reference: storage_id.clone().to_string(),
                        });
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
    } else {
        Ok(response)
    };
}

pub fn webhook_message(event: Event) -> Result<StandardResponse, StandardResponse> {
    trace!("{}", serde_json::to_string_pretty(&event).unwrap());

    let mut response: StandardResponse = StandardResponse::new();
    let mut errors: Vec<String> = vec![];
    let mut references = vec![];

    if event.entry[0].changes[0].value.messages.is_none() {
        error!("Not a user message");
        errors.push("Not a user message".to_string());

        response.errors = Some(errors);
        return  Err(response)
    }

    let phone_number = &event.entry[0].changes[0].value.messages.as_ref().unwrap()[0]
        .from
        .clone();
    let message_id = &event.entry[0].changes[0].value.messages.as_ref().unwrap()[0]
        .id
        .clone();

    let message_reference = get_user_last_message(&phone_number).unwrap();

    let mut expired_message = false;
    if message_reference != "" {
        // Get user last message linked to previously obtained reference
        let message = get_user_message(message_reference, &phone_number).unwrap();

        // Check expiration time for user last message
        let time_as_integer = &message.entry[0].changes[0].value.messages.as_ref().unwrap()[0]
            .timestamp
            .parse::<i64>()
            .unwrap();

        let parsed_time = OffsetDateTime::from_unix_timestamp(*time_as_integer)
            .unwrap()
            .to_offset(offset!(-3));

        let time_difference = SystemTime::now().duration_since(SystemTime::from(parsed_time));

        trace!(
            "since last message: {} secs",
            time_difference.as_ref().unwrap().as_secs()
        );

        // If message was 6 hours or more ago
        if time_difference.unwrap().as_secs() > 21600 {
            // reset user mode to 0
            set_user_mode(phone_number, "100");
            expired_message = true;
        };
    }

    let mode = get_user_mode(phone_number).unwrap();

    // Get mode destination systems
    let destination_system = get_destination_system(mode);

    // Store json message on redis
    let json_result = store_message(
        &event.clone(),
        phone_number,
        message_id,
        "incoming-messages",
    );

    match json_result {
        Ok(_) => references.push(ModifiedReference {
            system: "REDIS".to_string(),
            reference: format!("incoming-messages:{}:{}", phone_number, message_id),
        }),
        Err(err) => errors.push(format!("{}", err)),
    }

    // Build notification log
    let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => n.as_millis().to_string(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    };

    let log = MessageLog {
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
            references.push(ModifiedReference {
                system: "REDIS".to_string(),
                reference: format!("whatsapp-notification:{}", phone_number),
            });

            // Store notification sent to channel
            let id = log_message(&log);

            match id {
                Ok(redis_id) => {
                    references.push(ModifiedReference {
                        system: "REDIS".to_string(),
                        reference: redis_id,
                    });

                    // Set message id as last message
                    let res = set_last_message(&message_id, &phone_number).unwrap();
                }
                Err(err) => errors.push(format!("{}", err)),
            }
        }
        Err(err) => errors.push(format!("{}", err)),
    };

    // Build response
    response.references = references;

    if errors.len() > 0 {
        response.errors = Some(errors);
        Err(response)
    } else {
        Ok(response)
    }
}

pub fn send_menu(log: MessageLog) -> Result<StandardResponse, StandardResponse> {
    let mut response: StandardResponse = StandardResponse::new();
    let mut errors: Vec<String> = vec![];
    let mut references = vec![];
    let timestamp2 = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => n.as_millis().to_string(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    };

    // If user has no mode set(mode 0)
    let mode = get_user_mode(&log.phone_number).unwrap();

    info!("current mode: {}", &mode);

    // Check if user has an expired message or has never sent a message before
    if mode == 100 {
        let request = MessageRequest {
            system_id: 1,
            to: vec![String::from(&log.phone_number)],
            message_type: "text".to_string(),
            content: MessageContent {
                body: Some(
                    "Opciones disponibles:\n 1. Busqueda respuesto.\n 2. Ayuda.".to_string(),
                ),
                list: None,
                buttons: None,
            },
        };

        set_user_mode(&log.phone_number, "0");
        send_message(request);

        return Ok(response);
    }

    // Get user last message id
    let ws_message_id = get_user_last_message(&log.phone_number);

    if ws_message_id.is_err() {
        errors.push(ws_message_id.unwrap_err().to_string());
        response.references = references;
        response.errors = None;

        return Err(response);
    }

    info!("User last message id: {}", ws_message_id.as_ref().unwrap());

    // Get user last message content
    let ws_message: Result<Event, RedisError> = get_user_message(
        ws_message_id.as_ref().unwrap().to_string(),
        &log.phone_number,
    );

    if ws_message.is_err() {
        errors.push("Error obtaining user message".to_string());
        response.references = references;
        response.errors = Some(errors);

        return Err(response);
    }

    // Check if message type is a text message
    let message_type = String::from(
        &ws_message.as_ref().unwrap().entry[0].changes[0]
            .value
            .messages
            .as_ref()
            .unwrap()[0]
            .message_type,
    );

    info!("message Type: {}", &message_type);

    // Returns error is user send a non plain text message
    if message_type != "text" {
        errors.push("Message type has to be a text message, with only the number of the mode to be selected.".to_string());

        let request = MessageRequest{
            system_id: 1,
            to: vec![log.phone_number],
            message_type: "text".to_string(),
            content: MessageContent {
                body: Some("1. La opcion ingresada no es valida, debe ingresar solamente el numero de la opcion a seleccionar, intente nuevamente.".to_string()),
                list: None,
                buttons: None,
            },
        };

        send_message(request);
        response.references = references;
        response.errors = Some(errors);

        return Err(response);
    }


    // MODE MANAGEMENT

    // Check if user wanna change mode
    info!("mode: {}", mode);
    info!("content: {}", ws_message.as_ref().unwrap().entry[0].changes[0]
        .value
        .messages
        .as_ref()
        .unwrap()[0]
        .text
        .as_ref()
        .unwrap()
        .body
        .to_lowercase());
    if mode != 0
        && ws_message.as_ref().unwrap().entry[0].changes[0]
        .value
        .messages
        .as_ref()
        .unwrap()[0]
        .text
        .as_ref()
        .unwrap()
        .body
        .to_lowercase()
        == "salir"
    {
        info!("User exiting mode {}", &mode);

        set_user_mode(&log.phone_number, "100");
        send_menu(log.clone());

        response.references = references;
        response.errors = None;
        return Ok(response);
    }

    // check if message is a number
    let option = ws_message.as_ref().unwrap().entry[0].changes[0]
        .value
        .messages
        .as_ref()
        .unwrap()[0]
        .text
        .as_ref()
        .unwrap()
        .body
        .parse::<u8>();

    // Send error is message cant be parsed to a number
    if option.is_err() {
        errors.push("La opcion ingresada no es valida, debe ingresar solamente el numero de la opcion a seleccionar, intente nuevamente.".to_string());

        let request = MessageRequest{
            system_id: 1,
            to: vec![log.phone_number],
            message_type: "text".to_string(),
            content: MessageContent {
                body: Some("La opcion ingresada no es valida, debe ingresar solamente el numero de la opcion a seleccionar, intente nuevamente.".to_string()),
                list: None,
                buttons: None,
            },
        };

        send_message(request);
        response.references = references;
        response.errors = Some(errors);

        return Err(response);
    }

    // Unwraps parsed number
    let option_number = option.unwrap();

    info!("Option selected: {}", &option_number);

    // Get destination systems for the selected number
    let systems = get_destination_system(option_number as u16);

    // Error obtaining mode destination systems
    if systems.is_err() {
        errors.push(systems.as_ref().unwrap_err().to_string());

        response.references = references;
        response.errors = Some(errors);

        return Err(response);
    }

    info!("mode {} systems: {:?}",option_number,systems.as_ref().unwrap());

    // error is destination systems for mode is an empty list
    if systems.as_ref().unwrap().is_empty() {
        errors.push("El modo seleccionado no se encuentra entre las opciones disponibles, selecciona un modo listado.".to_string());
        let request = MessageRequest{
            system_id: 1,
            to: vec![log.phone_number.clone()],
            message_type: "text".to_string(),
            content: MessageContent {
                body: Some("El modo seleccionado no se encuentra entre las opciones disponibles, selecciona un modo listado.".to_string()),
                list: None,
                buttons: None,
            },
        };

        send_message(request);

        response.references = references;
        response.errors = Some(errors);

        return Err(response);
    }


    // If user is in mode selection
    if mode == 0{

        info!("Processing user option selection");

        // Set user new mode
        set_user_mode(&log.phone_number, &option_number.to_string());

        // Notify user selection
        let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(n) => n.as_millis().to_string(),
            Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        };

        // Notify selection successful
        let notification_log = MessageLog {
            timestamp: timestamp,
            destination_systems: systems.as_ref().unwrap().clone(),
            phone_number: String::from(&log.phone_number),
            origin: "OUTGOING".to_string(),
            register_id: ws_message_id.as_ref().unwrap().clone(),
        };

        publish_message(&notification_log, &log.phone_number);

        let request = MessageRequest{
            system_id: 1,
            to: vec![log.clone().phone_number],
            message_type: "text".to_string(),
            content: MessageContent {
                body: Some(format!("Ha seleccionado el opcion {}, si desea seleccionar otra opcion esriba 'salir' en el chat.", option_number)),
                list: None,
                buttons: None,
            },
        };

        send_message(request);
    }

    // If there a system mode selected
    if mode != 0 && mode !=100 {

        info!("Sending message to user selected option system");

        let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(n) => n.as_millis().to_string(),
            Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        };

        // Notify selection successful
        let notification_log = MessageLog {
            timestamp: timestamp,
            destination_systems: systems.as_ref().unwrap().clone(),
            phone_number: String::from(&log.clone().phone_number),
            origin: "INCOMING".to_string(),
            register_id: ws_message_id.as_ref().unwrap().clone(),
        };

        publish_message(&notification_log, &log.phone_number);

    }

    response.references = references;
    response.errors = None;

    Ok(response)

}
