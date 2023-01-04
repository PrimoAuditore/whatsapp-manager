use serde::*;
use serde_derive::{Deserialize, Serialize};

pub mod webhooks {

    use crate::structs::Storable;
    use serde_derive::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Event {
        object: String,
        pub(crate) entry: Vec<Entry>,
    }

    impl Storable for Event {}

    #[derive(Serialize, Deserialize)]
    pub struct MediaData {
        pub url: String,
        pub mime_type: String,
        pub sha256: String,
        pub file_size: i32,
        pub id: String,
        pub messaging_product: String,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Entry {
        id: String,
        pub(crate) changes: Vec<Change>,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Change {
        field: String,
        pub(crate) value: ChangeValue,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct ChangeValue {
        messaging_product: String,
        metadata: ChangeMetadata,
        contacts: Option<Vec<Contact>>,
        pub(crate) messages: Option<Vec<Message>>,
        pub statuses: Option<Vec<Status>>,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Status {
        id: String,
        status: String,
        timestamp: String,
        recipient_id: String,
        conversation: Option<Conversation>,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Conversation {
        id: String,
        origin: Origin,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Origin {
        #[serde(alias = "type")]
        origin_type: String,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct ChangeMetadata {
        display_phone_number: String,
        phone_number_id: String,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Contact {
        profile: Profile,
        wa_id: String,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Message {
        pub(crate) context: Option<Context>,
        pub(crate) from: String,
        pub(crate) id: String,
        pub(crate) timestamp: String,

        #[serde(alias = "type")]
        pub(crate) message_type: String,
        pub image: Option<Image>,
        pub(crate) text: Option<Text>,
        pub(crate) button: Option<Button>,
        pub(crate) interactive: Option<Interactive>,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Image {
        pub caption: String,
        pub mime_type: String,
        pub sha256: String,
        pub id: String,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Interactive {
        #[serde(alias = "type")]
        interactive_type: String,
        pub(crate) list_reply: Option<ListReply>,
        pub button_reply: Option<ListReply>,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct ListReply {
        pub(crate) id: String,
        title: String,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Button {
        payload: String,
        pub(crate) text: String,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Context {
        from: String,
        id: String,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Profile {
        name: String,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Text {
        pub(crate) body: String,
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StandardResponse {
    pub references: Vec<ModifiedReference>,
    pub errors: Option<Vec<String>>,
}

impl StandardResponse {
    pub fn new() -> StandardResponse {
        StandardResponse {
            references: vec![],
            errors: None,
        }
    }
}

pub trait Storable {}

#[derive(Serialize, Deserialize, Clone)]
pub struct ModifiedReference {
    pub(crate) system: String,
    pub(crate) reference: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MessageLog {
    pub timestamp: String,
    pub destination_systems: Vec<String>,
    pub phone_number: String,
    pub origin: String,
    pub register_id: String,
}

impl Storable for MessageLog {}
