use crate::structs::webhooks;
use log::{debug, error};
use serde_derive::{Deserialize, Serialize};
use std::error::Error;
use std::time::Duration;
use ureq::Agent;

// #[derive(Serialize, Deserialize, Clone)]
// pub struct MessageRequest {
//     pub system_id: u8,
//     pub to: Vec<String>,
//     #[serde(rename(serialize = "type"))]
//     pub message_type: String,
//     pub content: MessageContent,
// }
#[derive(Serialize, Deserialize, Clone)]
pub struct MessageContent {
    pub body: Option<String>,
    pub list: Option<ListMessage>,
    pub buttons: Option<ButtonMessage>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ListMessage {
    pub title: String,
    pub choices: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ButtonMessage {
    pub title: String,
    pub choices: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WhatsappRequest {
    messaging_product: String,
    recipient_type: String,
    to: String,
    #[serde(rename(serialize = "type"))]
    message_type: String,
    text: Option<webhooks::Text>,
    interactive: Option<InteractiveDefinition>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InteractiveDefinition {
    #[serde(rename(serialize = "type"))]
    interactive_type: String,
    body: Body,
    action: Action,
    header: Header,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Header {
    #[serde(rename(serialize = "type"))]
    header_type: String,
    text: String,
}

impl Default for InteractiveDefinition {
    fn default() -> Self {
        Self {
            interactive_type: "".to_string(),
            body: Body {
                text: "".to_string(),
            },
            action: Action {
                buttons: Some(vec![]),
                button: Some("".to_string()),
                sections: Some(vec![Section {
                    title: "".to_string(),
                    rows: vec![],
                }]),
            },
            header: Header {
                header_type: "text".to_string(),
                text: "".to_string(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Body {
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Action {
    buttons: Option<Vec<Button>>,
    button: Option<String>,
    sections: Option<Vec<Section>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Section {
    title: String,
    rows: Vec<Row>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Row {
    title: String,
    id: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Button {
    #[serde(rename(serialize = "type"))]
    button_type: String,
    reply: Reply,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Reply {
    id: String,
    title: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MessageBuilder {
    request: WhatsappRequest,
}

pub enum MessageType {
    Text,
    Interactive,
    InteractiveButton,
    InteractiveList,
}

impl MessageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::Text => "text",
            MessageType::Interactive => "interactive",
            MessageType::InteractiveButton => "button",
            MessageType::InteractiveList => "list",
        }
    }

    pub fn from_str(message_type: &str) -> MessageType {
        match message_type {
            "text" => MessageType::Text,
            "interactive" => MessageType::Interactive,
            "button" => MessageType::InteractiveButton,
            "list" => MessageType::InteractiveList,
            _ => {
                error!(
                    "{}",
                    format!("Message type {} was not found", String::from(message_type)).as_str()
                );
                panic!(
                    "{}",
                    format!("Message type {} was not found", String::from(message_type)).as_str()
                )
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MessageResponse {
    pub messaging_product: String,
    pub contacts: Vec<Contact>,
    pub messages: Vec<MessageReference>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Contact {
    pub input: String,
    pub wa_id: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MessageReference {
    pub id: String,
}

impl MessageBuilder {
    pub fn new() -> MessageBuilder {
        MessageBuilder::default()
    }

    pub fn execute(&self) -> Result<MessageResponse, Box<dyn Error>> {
        debug!("{}", ureq::json!(&self.request));
        let resp = ureq::post("https://graph.facebook.com/v15.0/110000391967238/messages")
            .set(
                "Authorization",
                format!("Bearer {}", std::env::var("META_TOKEN").unwrap()).as_str(),
            )
            .send_json(ureq::json!(&self.request))?
            .into_string();

        println!("{:?}", resp);

        match resp {
            Ok(response_body) => {
                let parsed_response: Result<MessageResponse, _> =
                    serde_json::from_str(response_body.as_str());

                if parsed_response.is_err() {
                    error!("{}", format!("Couldnt parse element: {}", response_body));
                    panic!("{}", format!("Couldnt parse element: {}", response_body))
                }

                Ok(parsed_response.unwrap())
            }
            Err(err) => {
                error!("{}", format!("{}", err.to_string()));
                panic!("{}", format!("{}", err.to_string()))
            }
        }
    }

    pub fn message_type(
        &mut self,
        message_type: MessageType,
        composed_type: Option<MessageType>,
    ) -> &mut MessageBuilder {
        // Check for primary types
        match &message_type {
            MessageType::InteractiveButton | MessageType::InteractiveList => {
                error!("Secondary types are not allowed, please use either text or interactive");
                panic!("Secondary types are not allowed, please use either text or interactive")
            }
            _ => {}
        }
        // Set primary type
        self.request.message_type = message_type.as_str().to_string();

        // Set secondary type if provided
        if composed_type.is_some() && message_type.as_str() == MessageType::Interactive.as_str() {
            self.request.interactive = Some(InteractiveDefinition {
                interactive_type: composed_type.unwrap().as_str().to_string(),
                ..Default::default()
            })
        }

        self
    }

    pub fn body(&mut self, body: String) -> &mut MessageBuilder {
        // Check if message type is already set
        if self.request.message_type == "" {
            error!("primary type is not set, please call the message_type method and set a value");
            panic!("primary type is not set, please call the message_type method and set a value")
        }

        match MessageType::from_str(&self.request.message_type) {
            MessageType::Text => {
                self.request.text = Some(webhooks::Text { body });
            }
            MessageType::Interactive
            | MessageType::InteractiveButton
            | MessageType::InteractiveList => {
                let mut clone = self.request.interactive.clone();
                clone.as_mut().unwrap().body.text = body;

                self.request.interactive = clone;
            }
        }

        self
    }

    pub fn main_button_content(&mut self, button_name: String) -> &mut MessageBuilder {

        self.request.interactive.as_mut().unwrap().action.button = Some(button_name);

        self
    }

    pub fn header(&mut self, header: String) -> &mut MessageBuilder {
        // Check if message type is already set
        if self.request.message_type == "" {
            error!("primary type is not set, please call the message_type method and set a value");
            panic!("primary type is not set, please call the message_type method and set a value")
        }

        match MessageType::from_str(&self.request.message_type) {
            MessageType::Text => {
                error!("text messages doesn't allow header");
                panic!("text messages doesn't allow header")
            }
            MessageType::Interactive
            | MessageType::InteractiveButton
            | MessageType::InteractiveList => {
                let mut clone = self.request.interactive.clone();
                clone.as_mut().unwrap().header.text = header;

                self.request.interactive = clone;
            }
        }

        self
    }

    pub fn to(&mut self, phone_number: String) -> &mut MessageBuilder {
        // TODO: Check for phone number validation
        self.request.to = phone_number;
        self
    }
    pub fn add_reply_button(
        &mut self,
        button_content: &str,
        button_id: Option<&str>,
    ) -> &mut MessageBuilder {
        let mut copy = self.request.clone();
        if self.request.message_type == "text" {
            error!("Text message type doesnt allow actions");
            panic!("Text message type doesnt allow actions")
        }

        match MessageType::from_str(&self.request.interactive.as_ref().unwrap().interactive_type) {
            MessageType::InteractiveButton => {
                let default = format!("{}-id", button_content.to_lowercase().replace(" ", "-"));
                let button_id_str = button_id.unwrap_or(default.as_str());
                let button = Button {
                    button_type: "reply".to_string(),
                    reply: Reply {
                        id: button_id_str.to_string(),
                        title: button_content.to_string(),
                    },
                };
                // self.request.interactive.as_ref().unwrap().action.buttons.as_ref().unwrap().push(button);
                copy.interactive
                    .as_mut()
                    .unwrap()
                    .action
                    .buttons
                    .as_mut()
                    .unwrap()
                    .push(button);
            }
            MessageType::InteractiveList => {
                error!("Invalid method for message type, use add_list_button method instead");
                panic!("Invalid method for message type, use add_list_button method instead")
            }
            _ => {}
        }

        self.request = copy;

        self
    }

    pub fn set_button_title(&mut self, button_title: &str) -> &mut MessageBuilder {
        if &self.request.message_type != MessageType::InteractiveList.as_str() {
            error!("To set button title, message type must be InteractiveList");
            panic!("To set button title, message type must be InteractiveList");
        }

        self.request.clone().interactive.unwrap().action.button = Some(button_title.to_string());
        self
    }

    pub fn add_list_button(
        &mut self,
        button_content: &str,
        button_id: Option<&str>,
        button_name: &str,
    ) -> &mut MessageBuilder {

        let mut copy = self.request.clone();
        if self.request.message_type == "text" {
            error!("Text message type doesnt allow actions");
            panic!("Text message type doesnt allow actions")
        }

        match MessageType::from_str(&self.request.interactive.as_ref().unwrap().interactive_type) {
            MessageType::InteractiveList => {
                let default = format!("{}-id", button_content.to_lowercase().replace(" ", "-"));
                let button_id_str = button_id.unwrap_or(default.as_str());

                // Check if section exists, if not creates a section with an empty list of buttons
                if copy.interactive.as_ref().unwrap().action.sections.is_none() {

                    let section = Some(vec![Section{
                        title: "".to_string(),
                        rows: vec![],
                    }]);


                    copy.interactive
                        .as_mut()
                        .unwrap()
                        .action
                        .sections = section;
                }

                let row = Row{
                    id: button_id_str.to_string(),
                    title: button_content.to_string(),
                };


                &copy.interactive.as_mut().unwrap().action.sections.as_mut().unwrap()[0].rows.push(row);



            }
            MessageType::InteractiveButton => {
                error!("Invalid method for message type, use add_list_button method instead");
                panic!("Invalid method for message type, use add_list_button method instead")
            }
            _ => {}
        }

        self.request = copy;

        self
    }
}

impl Default for MessageBuilder {
    fn default() -> Self {
        MessageBuilder {
            request: WhatsappRequest {
                messaging_product: "whatsapp".to_string(),
                recipient_type: "individual".to_string(),
                to: "".to_string(),
                message_type: "".to_string(),
                text: None,
                interactive: None,
            },
        }
    }
}
