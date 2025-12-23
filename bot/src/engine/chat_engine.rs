use teloxide::{
    Bot,
    dispatching::{UpdateFilterExt, dialogue::InMemStorage},
    dptree::{self},
    prelude::{Dispatcher, Requester},
    types::{Message, Update},
    utils::markdown,
};

use crate::engine::{
    command::Command,
    command_handler::build_command_handler,
    dialogue_handler::{TaskState, build_dialogue_callback_handler, build_dialogue_handler},
    utils::{ChatHandlerResult, send_chat_message_markdown},
};

pub struct ChatEngine {
    bot: Bot,
}

impl ChatEngine {
    pub fn new(bot: Bot) -> Self {
        ChatEngine { bot }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Chat engine is running...");

        let bot_username = self
            .bot
            .get_me()
            .await?
            .username.clone()
            .ok_or("Username not found")?;

        let command_handler = build_command_handler();

        let mention_handler = Update::filter_message()
            .filter(move |msg: Message| is_bot_mentioned(&msg, &bot_username))
            .endpoint(bot_mentioned_handler);

        let dialogue_handler = build_dialogue_handler();
        let dialogue_callback_handler = build_dialogue_callback_handler();

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

fn is_bot_mentioned(msg: &Message, bot_username: &str) -> bool {
    if let Some(entities) = msg.entities() {
        for entity in entities {
            if let teloxide::types::MessageEntityKind::Mention = entity.kind
                && let Some(text) = msg.text() {
                    let mention =
                        &text[entity.offset..(entity.offset + entity.length)];
                    if mention == format!("@{}", bot_username) {
                        return true;
                    }
                }
        }
    }

    false
}

fn get_user_mention(msg: &Message) -> String {
    let mut username = String::new();

    if let Some(user) = &msg.from {
        if let Some(uname) = &user.username {
            username = format!("@{}", markdown::escape(uname));
        } else {
            let display_name = markdown::escape(&user.first_name);
            username = format!("[{}](tg://user?id={})", display_name, user.id);
        }
    }

    if !username.is_empty() {
        return format!(" {}", username);
    }

    username
}

async fn bot_mentioned_handler(bot: Bot, msg: Message) -> ChatHandlerResult {
    let user_mention = get_user_mention(&msg);

    let title = format!(
        "Nešto si trebao brate{}? Evo šta mogu da ponudim:",
        user_mention
    );

    let command_list = markdown::escape(&Command::get_command_list());
    let message = format!("{}\n\n{}", title, command_list);

    send_chat_message_markdown(&bot, msg.chat.id, message).await;

    Ok(())
}
