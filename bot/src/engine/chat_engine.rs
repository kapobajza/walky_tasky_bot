use scheduler::task::task_scheduler::TaskScheduler;
use teloxide::{
    Bot,
    dispatching::dialogue::InMemStorage,
    dptree::{self},
    prelude::{Dispatcher, Requester},
};

use crate::engine::{
    bot_mentioned_handler::build_bot_mentioned_handler,
    command_handler::build_command_handler,
    dialogue_handler::{TaskState, build_dialogue_callback_handler, build_dialogue_handler},
};

pub struct ChatEngine {
    bot: Bot,
    scheduler: TaskScheduler,
}

impl ChatEngine {
    pub fn new(bot: Bot, scheduler: TaskScheduler) -> Self {
        ChatEngine { bot, scheduler }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Chat engine is running...");

        let bot_username = self
            .bot
            .get_me()
            .await?
            .username
            .clone()
            .ok_or("Username not found")?;

        let command_handler = build_command_handler();
        let mention_handler = build_bot_mentioned_handler(bot_username);

        let dialogue_handler = build_dialogue_handler();
        let dialogue_callback_handler = build_dialogue_callback_handler(self.scheduler.clone());

        let handler = dptree::entry()
            .branch(command_handler)
            .branch(dialogue_handler)
            .branch(dialogue_callback_handler)
            .branch(mention_handler);

        Dispatcher::builder(self.bot.clone(), handler)
            .dependencies(dptree::deps![InMemStorage::<TaskState>::new()])
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;

        Ok(())
    }
}
