use chrono_tz::Europe::Sarajevo;
use scheduler::task::{action::TaskAction, default::Task, task_scheduler::TaskScheduler};
use teloxide::{
    Bot,
    types::{Message, MessageEntityKind},
    utils::markdown,
};

use crate::engine::{
    dialogue_handler::{TaskDialogue, TaskState},
    utils::{
        CALENDAR_DEFAULT_DATE_FORMAT, ChatHandlerResult, TIME_DEFAULT_FORMAT, send_chat_message,
        send_chat_message_markdown,
    },
};

pub async fn handle_assigne_mention_callback(
    bot: Bot,
    msg: Message,
    dialogue: TaskDialogue,
    scheduler: &TaskScheduler,
) -> ChatHandlerResult {
    let state = dialogue.get().await?.ok_or("Dialogue state not found")?;

    let (task_name, date, time) = match state {
        TaskState::AwaitingAssigneeMention {
            task_name,
            date,
            time,
        } => (task_name, date, time),
        _ => return Err("Invalid dialogue state".into()),
    };

    let user_mention: Option<String> = if let Some(entities) = msg.entities() {
        let mut found_mention = None;

        for entity in entities {
            match &entity.kind {
                MessageEntityKind::Mention => {
                    if let Some(mention_text) = msg
                        .text()
                        .and_then(|t| t.get(entity.offset..entity.offset + entity.length))
                    {
                        found_mention = Some(mention_text.to_string());
                        break;
                    }
                }
                MessageEntityKind::TextMention { user } => {
                    let display_name = if let Some(username) = &user.username {
                        format!("@{}", username)
                    } else {
                        user.first_name.clone()
                    };
                    found_mention = Some(format!(
                        "[{}](tg://user?id={})",
                        markdown::escape(&display_name),
                        user.id
                    ));
                    break;
                }
                _ => {}
            }
        }

        found_mention
    } else {
        None
    };

    match user_mention {
        Some(mention) => {
            let next_run = chrono::NaiveDateTime::parse_from_str(
                &format!("{} {}", date, time),
                &format!("{} {}", CALENDAR_DEFAULT_DATE_FORMAT, TIME_DEFAULT_FORMAT),
            )?;
            let next_run = next_run
                .and_local_timezone(Sarajevo)
                .single()
                .ok_or("Failed to convert to timezone-aware datetime")?
                .with_timezone(&chrono::Utc);

            scheduler
                .add_task(Task::new_with_datetime(
                    next_run,
                    TaskAction::SendBotMessage {
                        chat_id: msg.chat.id.0,
                        message: format!("Podsjetnik za zadatak: {}", task_name),
                    },
                ))
                .await?;
            let confirmation_message = format!(
                "Zadatak '{}' je dodijeljen korisniku {} za {} u {}\\.",
                markdown::escape(&task_name),
                mention,
                markdown::escape(&date),
                time
            );
            send_chat_message_markdown(&bot, msg.chat.id, confirmation_message).await;
            dialogue.exit().await?;
        }
        None => {
            send_chat_message(
                &bot,
                msg.chat.id,
                "Nisi spomenuo korisnika. Molim te pokušaj ponovno.".to_string(),
            )
            .await;
        }
    }

    Ok(())
}
