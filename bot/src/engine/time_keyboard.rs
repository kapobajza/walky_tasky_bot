use chrono::{NaiveTime, Timelike};
use chrono_tz::Europe::Sarajevo;
use scheduler::task::{action::TaskAction, default::Task, task_scheduler::TaskScheduler};
use teloxide::{
    Bot,
    prelude::Requester,
    types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup},
};

use crate::engine::{
    dialogue_handler::{TaskDialogue, TaskState},
    utils::{CALENDAR_DEFAULT_DATE_FORMAT, ChatHandlerResult, TIME_DEFAULT_FORMAT},
};

pub static TIME_SELECTION_CALLBACK_PREFIX: &str = "time_select_";
pub static TIME_SELECTION_CANCEL: &str = "time_cancel";
pub static TIME_SELECTION_IGNORE: &str = "time_ignore";
pub static TIME_SELECTION_PAGE_PREFIX: &str = "time_page_";

pub fn create_time_selection_keyboard(
    min_time: Option<NaiveTime>,
    page: usize,
) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = vec![];
    let min_minutes = min_time.map(|t| t.hour() * 60 + t.minute());

    let slots_per_page = 16;
    let total_slots = 24 * 4;
    // let start_slot = page * slots_per_page;
    // let end_slot = ((page + 1) * slots_per_page).min(total_slots);

    let available_slots: Vec<usize> = (0..total_slots)
        .filter(|&slot| {
            let total_minutes = (slot / 4 * 60 + (slot % 4) * 15) as u32;
            !min_minutes.map(|m| total_minutes < m).unwrap_or(false)
        })
        .collect();
    let total_available = available_slots.len();
    let total_pages = total_available.div_ceil(slots_per_page);
    let page = page.min(total_pages - 1);
    let start_idx = page * slots_per_page;
    let end_idx = ((page + 1) * slots_per_page).min(total_available);

    for (i, &slot) in available_slots[start_idx..end_idx].iter().enumerate() {
        let hour = slot / 4;
        let minute = (slot % 4) * 15;

        let button = InlineKeyboardButton::callback(
            format!("{:02}:{:02}", hour, minute),
            format!(
                "{}{:02}:{:02}",
                TIME_SELECTION_CALLBACK_PREFIX, hour, minute
            ),
        );

        if i % 4 == 0 {
            rows.push(vec![button]);
        } else if let Some(last_row) = rows.last_mut() {
            last_row.push(button);
        }
    }

    // Navigation row
    let mut nav_row = vec![];
    if page > 0 {
        nav_row.push(InlineKeyboardButton::callback(
            "◀️",
            format!("{}{}", TIME_SELECTION_PAGE_PREFIX, page - 1),
        ));
    } else {
        nav_row.push(InlineKeyboardButton::callback(" ", TIME_SELECTION_IGNORE));
    }
    nav_row.push(InlineKeyboardButton::callback(
        "❌ Odustani",
        TIME_SELECTION_CANCEL,
    ));
    if page + 1 < total_slots {
        nav_row.push(InlineKeyboardButton::callback(
            "▶️",
            format!("{}{}", TIME_SELECTION_PAGE_PREFIX, page + 1),
        ));
    } else {
        nav_row.push(InlineKeyboardButton::callback(" ", TIME_SELECTION_IGNORE));
    }
    rows.push(nav_row);

    InlineKeyboardMarkup::new(rows)
}

pub async fn handle_keyboard_time_selection(
    bot: Bot,
    chat_id: ChatId,
    dialogue: TaskDialogue,
    state: TaskState,
    time_str: &str,
    scheduler: &TaskScheduler,
) -> ChatHandlerResult {
    match state {
        TaskState::AwaitingSpecificTime { task_name, date } => {
            let next_run = chrono::NaiveDateTime::parse_from_str(
                &format!("{} {}", date, time_str),
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
                        chat_id: chat_id.0,
                        message: format!("Podsjetnik za zadatak: {}", task_name),
                    },
                ))
                .await?;
            bot.send_message(
                chat_id,
                format!(
                    "Zadatak \"{}\" je zakazan {} u {}.",
                    task_name, date, time_str
                ),
            )
            .await?;
        }
        TaskState::AwaitingRecurringTime {
            task_name,
            start_date,
            end_date,
        } => {
            bot.send_message(
                chat_id,
                format!(
                    "Hvala! Zakazujem ponavljajući zadatak \"{}\" od {} do {} u {} svaki dan.",
                    task_name, start_date, end_date, time_str
                ),
            )
            .await?;
        }
        _ => {}
    }

    dialogue.exit().await?;

    Ok(())
}
