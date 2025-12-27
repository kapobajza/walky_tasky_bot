use std::error::Error;

use teloxide::{
    Bot,
    dispatching::{DpHandlerDescription, UpdateFilterExt},
    dptree::Handler,
    types::{Message, Update},
    utils::markdown,
};

use crate::engine::{
    command::Command,
    utils::{ChatHandlerResult, get_user_mention, send_chat_message_markdown},
};

pub fn is_bot_mentioned(msg: &Message, bot_username: &str) -> bool {
    if let Some(entities) = msg.entities() {
        for entity in entities {
            if let teloxide::types::MessageEntityKind::Mention = entity.kind
                && let Some(text) = msg.text()
            {
                let mention = &text[entity.offset..(entity.offset + entity.length)];
                if mention == format!("@{}", bot_username) {
                    return true;
                }
            }
        }
    }

    false
}

pub async fn bot_mentioned_handler(bot: Bot, msg: Message) -> ChatHandlerResult {
    let user_mention = {
        if let Some(mention) = get_user_mention(&msg) {
            format!(" {}", mention)
        } else {
            Err("Failed to get user mention")?
        }
    };

    let title = format!(
        "Nešto si trebao brate{}? Evo šta mogu da ponudim:",
        user_mention
    );

    let command_list = markdown::escape(&Command::get_command_list());
    let message = format!("{}\n\n{}", title, command_list);

    send_chat_message_markdown(&bot, msg.chat.id, message).await;

    Ok(())
}

pub fn build_bot_mentioned_handler(
    bot_username: String,
) -> Handler<'static, Result<(), Box<dyn Error + Send + Sync + 'static>>, DpHandlerDescription> {
    Update::filter_message()
        .filter(move |msg: Message| is_bot_mentioned(&msg, &bot_username))
        .endpoint(bot_mentioned_handler)
}
