use std::collections::HashMap;
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug)]
pub struct MonamiStatusMessage {
    pub hostname: String,
    pub uuid: String,
    pub tag: String,
    pub command: String,
    pub output: String,
    pub timestamp: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MonamiControlMessage {
    pub function: String,
    pub parameters: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageType { Invalid, Status, Control }

#[derive(Serialize, Deserialize, Debug)]
pub struct MonamiMessage {
    pub message_type: MessageType,
    pub message_status: Option<MonamiStatusMessage>,
    pub message_control: Option<MonamiControlMessage>,
}
