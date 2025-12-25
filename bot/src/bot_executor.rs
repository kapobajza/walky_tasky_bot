use async_trait::async_trait;
use scheduler::{
    error::SchedulerError,
    task::{
        action::{ActionType, TaskAction},
        action_executor::ActionExecutor,
        default::Task,
    },
};
use teloxide::{Bot, types::ChatId};

use crate::engine::utils::send_chat_message;

pub struct BotExecutor {
    bot: Bot,
}

impl BotExecutor {
    pub fn new(bot: Bot) -> Self {
        Self { bot }
    }
}

#[async_trait]
impl ActionExecutor for BotExecutor {
    fn supported_actions(&self) -> Vec<ActionType> {
        vec![ActionType::SendBotMessage]
    }

    async fn execute(&self, _task: &Task, action: &TaskAction) -> Result<(), SchedulerError> {
        if let TaskAction::SendBotMessage { chat_id, message } = action {
            send_chat_message(&self.bot, ChatId(*chat_id), message.clone()).await;
        }

        Ok(())
    }
}
