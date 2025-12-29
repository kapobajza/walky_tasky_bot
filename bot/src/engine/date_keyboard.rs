use chrono::{Datelike, NaiveDate};
use teloxide::{
    Bot,
    payloads::{EditMessageReplyMarkupSetters, SendMessageSetters},
    prelude::Requester,
    types::{CallbackQuery, ChatId, InlineKeyboardButton, InlineKeyboardMarkup},
};

use crate::engine::{
    dialogue_handler::{TaskDialogue, TaskState},
    time_keyboard::create_time_selection_keyboard,
    utils::{
        CALENDAR_DEFAULT_DATE_FORMAT, ChatHandlerResult, get_current_date_in_bosnia,
        get_current_time_in_bosnia,
    },
};

pub static CALENDAR_CALLBACK_SELECT_PREFIX: &str = "cal_select_";
pub static CALENDAR_CALLBACK_PREV_PREFIX: &str = "cal_prev_";
pub static CALENDAR_CALLBACK_NEXT_PREFIX: &str = "cal_next_";
pub static CALENDAR_CALLBACK_CANCEL: &str = "cal_cancel";
pub static CALENDAR_CALLBACK_IGNORE: &str = "cal_ignore";

pub fn create_calendar_keyboard(
    year: i32,
    month: u32,
    min_date: Option<NaiveDate>,
) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = vec![];
    let month_name = get_month_name(month);

    let can_go_prev = min_date
        .map(|d| year > d.year() || (year == d.year() && month > d.month()))
        .unwrap_or(true);

    rows.push(vec![
        InlineKeyboardButton::callback(
            if can_go_prev { "◀️" } else { " " },
            if can_go_prev {
                format!("{}{}_{}", CALENDAR_CALLBACK_PREV_PREFIX, year, month)
            } else {
                CALENDAR_CALLBACK_IGNORE.to_string()
            },
        ),
        InlineKeyboardButton::callback(
            format!("{} {}", month_name, year),
            CALENDAR_CALLBACK_IGNORE,
        ),
        InlineKeyboardButton::callback(
            "▶️",
            format!("{}{}_{}", CALENDAR_CALLBACK_NEXT_PREFIX, year, month),
        ),
    ]);

    // Day names header
    rows.push(vec![
        InlineKeyboardButton::callback("Pon", CALENDAR_CALLBACK_IGNORE),
        InlineKeyboardButton::callback("Uto", CALENDAR_CALLBACK_IGNORE),
        InlineKeyboardButton::callback("Sri", CALENDAR_CALLBACK_IGNORE),
        InlineKeyboardButton::callback("Čet", CALENDAR_CALLBACK_IGNORE),
        InlineKeyboardButton::callback("Pet", CALENDAR_CALLBACK_IGNORE),
        InlineKeyboardButton::callback("Sub", CALENDAR_CALLBACK_IGNORE),
        InlineKeyboardButton::callback("Ned", CALENDAR_CALLBACK_IGNORE),
    ]);

    // Days grid
    let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let days_in_month = get_days_in_month(year, month);
    let first_weekday = first_day.weekday().num_days_from_monday() as usize;

    let mut current_row: Vec<InlineKeyboardButton> = vec![];

    // Empty cells before first day
    for _ in 0..first_weekday {
        current_row.push(InlineKeyboardButton::callback(
            " ",
            CALENDAR_CALLBACK_IGNORE,
        ));
    }

    for day in 1..=days_in_month {
        let current_date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
        let is_disabled = min_date.map(|d| current_date < d).unwrap_or(false);

        let date_str = format!("{}.{:02}.{:02}", day, month, year);
        current_row.push(InlineKeyboardButton::callback(
            if is_disabled {
                "x".to_string()
            } else {
                day.to_string()
            },
            if is_disabled {
                CALENDAR_CALLBACK_IGNORE.to_string()
            } else {
                format!("cal_select_{}", date_str)
            },
        ));

        if current_row.len() == 7 {
            rows.push(current_row);
            current_row = vec![];
        }
    }

    // Fill remaining cells
    while current_row.len() < 7 && !current_row.is_empty() {
        current_row.push(InlineKeyboardButton::callback(
            " ",
            CALENDAR_CALLBACK_IGNORE,
        ));
    }
    if !current_row.is_empty() {
        rows.push(current_row);
    }

    rows.push(vec![InlineKeyboardButton::callback(
        "❌ Odustani",
        CALENDAR_CALLBACK_CANCEL,
    )]);

    InlineKeyboardMarkup::new(rows)
}

fn get_month_name(month: u32) -> &'static str {
    match month {
        1 => "Januar",
        2 => "Februar",
        3 => "Mart",
        4 => "April",
        5 => "Maj",
        6 => "Juni",
        7 => "Juli",
        8 => "August",
        9 => "Septembar",
        10 => "Oktobar",
        11 => "Novembar",
        12 => "Decembar",
        _ => "Unknown",
    }
}

fn get_days_in_month(year: i32, month: u32) -> u32 {
    NaiveDate::from_ymd_opt(year, month + 1, 1)
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap())
        .pred_opt()
        .unwrap()
        .day()
}

pub async fn handle_keyboard_date_selection(
    bot: Bot,
    chat_id: ChatId,
    dialogue: TaskDialogue,
    state: TaskState,
    date: NaiveDate,
) -> ChatHandlerResult {
    let min_time = if date == get_current_date_in_bosnia() {
        Some(get_current_time_in_bosnia())
    } else {
        None
    };

    match state {
        TaskState::AwaitingSpecificDate { task_name } => {
            bot.send_message(chat_id, "Odaberi vrijeme za zadatak:")
                .reply_markup(create_time_selection_keyboard(min_time, 0))
                .await?;
            dialogue
                .update(TaskState::AwaitingSpecificTime {
                    task_name,
                    date: date.format(CALENDAR_DEFAULT_DATE_FORMAT).to_string(),
                })
                .await?;
        }
        TaskState::AwaitingRangeStartDate { task_name } => {
            let now = get_current_date_in_bosnia();
            let start_date = date.format(CALENDAR_DEFAULT_DATE_FORMAT).to_string();
            let date_plus_one_day = date + chrono::Duration::days(1);

            bot.send_message(chat_id, "Odaberi završni datum za ponavljajući zadatak:")
                .reply_markup(create_calendar_keyboard(
                    now.year(),
                    now.month(),
                    Some(date_plus_one_day),
                ))
                .await?;
            dialogue
                .update(TaskState::AwaitingRangeEndDate {
                    task_name,
                    start_date,
                })
                .await?;
        }
        TaskState::AwaitingRangeEndDate {
            task_name,
            start_date,
        } => {
            bot.send_message(chat_id, "Odaberi vrijeme za zadatak:")
                .reply_markup(create_time_selection_keyboard(min_time, 0))
                .await?;
            dialogue
                .update(TaskState::AwaitingRangeTime {
                    task_name,
                    start_date,
                    end_date: date.format(CALENDAR_DEFAULT_DATE_FORMAT).to_string(),
                })
                .await?;
        }
        _ => {}
    }

    Ok(())
}

pub async fn handle_keyboard_calendar_navigation(
    bot: Bot,
    q: &CallbackQuery,
    data: &str,
) -> ChatHandlerResult {
    let parts: Vec<&str> = data.split('_').collect();
    if parts.len() >= 4 {
        let year: i32 = parts[2].parse().unwrap_or(2025);
        let month: u32 = parts[3].parse().unwrap_or(1);

        let (new_year, new_month) = if data.starts_with(CALENDAR_CALLBACK_PREV_PREFIX) {
            if month == 1 {
                (year - 1, 12)
            } else {
                (year, month - 1)
            }
        } else if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };

        if let Some(msg) = &q.message {
            bot.edit_message_reply_markup(msg.chat().id, msg.id())
                .reply_markup(create_calendar_keyboard(
                    new_year,
                    new_month,
                    Some(chrono::Utc::now().date_naive()),
                ))
                .await?;
        }
    }
    Ok(())
}
