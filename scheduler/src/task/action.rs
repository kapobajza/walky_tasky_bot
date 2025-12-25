use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionType {
    SendBotMessage,
    Log,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", content = "payload")]
pub enum TaskAction {
    SendBotMessage { chat_id: i64, message: String },
    Log { message: String, level: String },
}

impl TaskAction {
    pub fn action_type(&self) -> ActionType {
        match self {
            TaskAction::SendBotMessage { .. } => ActionType::SendBotMessage,
            TaskAction::Log { .. } => ActionType::Log,
        }
    }
}
