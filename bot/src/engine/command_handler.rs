use std::error::Error;

use teloxide::{
    Bot,
    dispatching::{DpHandlerDescription, HandlerExt, UpdateFilterExt, dialogue::InMemStorage},
    dptree::{self, Handler},
    prelude::Requester,
    types::{Message, Update},
};

use crate::engine::{
    command::Command,
    dialogue_handler::{TaskDialogue, TaskState},
    utils::{ChatHandlerResult, send_chat_message, send_welcome_message},
};

pub fn build_command_handler()
-> Handler<'static, Result<(), Box<dyn Error + Send + Sync + 'static>>, DpHandlerDescription> {
    Update::filter_message()
        .filter_command::<Command>()
        .enter_dialogue::<Message, InMemStorage<TaskState>, TaskState>()
        .branch(
            dptree::case![Command::Help].endpoint(|bot: Bot, msg: Message| {
                help_command_handler(bot, msg, "Evo šta mogu da radim:".to_string())
            }),
        )
        .branch(
            dptree::case![Command::Start]
                .endpoint(move |bot: Bot, msg: Message| start_command_handler(bot, msg)),
        )
        .branch(dptree::case![Command::NoviZadatak].endpoint(
            move |bot: Bot, msg: Message, dialogue: TaskDialogue| {
                new_task_command_handler(bot, msg, dialogue)
            },
        ))
}

async fn help_command_handler(bot: Bot, msg: Message, title: String) -> ChatHandlerResult {
    send_chat_message(
        &bot,
        msg.chat.id,
        format!("{}\n\n{}", title, Command::get_command_list()),
    )
    .await;

    Ok(())
}

async fn start_command_handler(bot: Bot, msg: Message) -> ChatHandlerResult {
    log::debug!("User started the bot: {:?}", msg.chat.id);

    send_welcome_message(bot, msg.chat.id).await;

    Ok(())
}

async fn new_task_command_handler(
    bot: Bot,
    msg: Message,
    dialogue: TaskDialogue,
) -> ChatHandlerResult {
    bot.send_message(
        msg.chat.id,
        "Unesite naziv zadatka koji želite da kreirate:",
    )
    .await?;

    dialogue.update(TaskState::AwaitingTaskName).await?;
    Ok(())
}
