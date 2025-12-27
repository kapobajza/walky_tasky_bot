use chrono::{Datelike, NaiveDate, TimeZone};
use chrono_tz::Europe::Sarajevo;
use scheduler::task::{action::TaskAction, default::Task, task_scheduler::TaskScheduler};
use teloxide::{
    Bot,
    dispatching::{DpHandlerDescription, HandlerExt, UpdateFilterExt, dialogue::InMemStorage},
    dptree::{self, Handler},
    payloads::{EditMessageReplyMarkupSetters, SendMessageSetters},
    prelude::{Dialogue, Requester},
    types::{CallbackQuery, ChatId, Message, Update},
};

use crate::engine::{
    date_time_keyboard::{
        CALENDAR_CALLBACK_CANCEL, CALENDAR_CALLBACK_NEXT_PREFIX, CALENDAR_CALLBACK_PREV_PREFIX,
        CALENDAR_CALLBACK_SELECT_PREFIX, TASK_TYPE_CANCEL_ID, TASK_TYPE_RECURRING_ID,
        TASK_TYPE_SPECIFIC_ID, TIME_SELECTION_CALLBACK_PREFIX, TIME_SELECTION_CANCEL,
        TIME_SELECTION_PAGE_PREFIX, create_calendar_keyboard, create_task_type_keyboard,
        create_time_selection_keyboard,
    },
    utils::{CALENDAR_DEFAULT_DATE_FORMAT, ChatHandlerResult, TIME_DEFAULT_FORMAT},
};

#[derive(Clone, Default, Debug)]
pub enum TaskState {
    #[default]
    Idle,
    AwaitingTaskName,
    AwaitingTaskType {
        task_name: String,
    },
    AwaitingSpecificDate {
        task_name: String,
    },
    AwaitingSpecificTime {
        task_name: String,
        date: String,
    },
    AwaitingRecurringStartDate {
        task_name: String,
    },
    AwaitingRecurringEndDate {
        task_name: String,
        start_date: String,
    },
    AwaitingRecurringTime {
        task_name: String,
        start_date: String,
        end_date: String,
    },
}

pub type TaskDialogue = Dialogue<TaskState, InMemStorage<TaskState>>;

pub fn build_dialogue_handler() -> Handler<
    'static,
    Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>,
    DpHandlerDescription,
> {
    Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<TaskState>, TaskState>()
        .branch(dptree::case![TaskState::AwaitingTaskName].endpoint(receive_task_name))
        .branch(
            dptree::case![TaskState::AwaitingSpecificDate { task_name }]
                .endpoint(receive_specific_date_text),
        )
        .branch(
            dptree::case![TaskState::AwaitingRecurringStartDate { task_name }]
                .endpoint(receive_recurring_start_text),
        )
}

pub fn build_dialogue_callback_handler(
    scheduler: TaskScheduler,
) -> Handler<
    'static,
    Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>,
    DpHandlerDescription,
> {
    Update::filter_callback_query()
        .enter_dialogue::<CallbackQuery, InMemStorage<TaskState>, TaskState>()
        .endpoint(move |bot: Bot, q: CallbackQuery, dialogue: TaskDialogue| {
            let scheduler = scheduler.clone();
            async move { handle_callback(bot, q, dialogue, &scheduler).await }
        })
}

async fn receive_task_name(bot: Bot, msg: Message, dialogue: TaskDialogue) -> ChatHandlerResult {
    if let Some(task_name) = msg.text() {
        bot.send_message(msg.chat.id, "Da li želiš da zakažeš zadatak na određeni datum i vrijeme ili kao ponavljajući zadatak? Određeni se izvršava samo jednom, dok se ponavljajući izvršava u određenim intervalima.")
            .reply_markup(create_task_type_keyboard())
            .await?;

        dialogue
            .update(TaskState::AwaitingTaskType {
                task_name: task_name.to_string(),
            })
            .await?;
    }

    Ok(())
}

async fn handle_callback(
    bot: Bot,
    q: CallbackQuery,
    dialogue: TaskDialogue,
    scheduler: &TaskScheduler,
) -> ChatHandlerResult {
    if let Some(data) = &q.data {
        let chat_id = q
            .message
            .as_ref()
            .map(|m| m.chat().id)
            .ok_or("No chat id")?;
        let state = dialogue.get().await?.unwrap_or_default();

        match data.as_str() {
            s if s == TASK_TYPE_SPECIFIC_ID => {
                if let TaskState::AwaitingTaskType { task_name } = state {
                    let now = get_current_date_in_bosnia();

                    remove_keyboard_buttons(&bot, &q).await;
                    bot.send_message(chat_id, "Odaberi datum za zadatak:")
                        .reply_markup(create_calendar_keyboard(now.year(), now.month(), Some(now)))
                        .await?;
                    dialogue
                        .update(TaskState::AwaitingSpecificDate { task_name })
                        .await?;
                }
            }
            s if s == TASK_TYPE_RECURRING_ID => {
                if let TaskState::AwaitingTaskType { task_name } = state {
                    let now = get_current_date_in_bosnia();

                    remove_keyboard_buttons(&bot, &q).await;
                    bot.send_message(chat_id, "Odaberi početni datum za ponavljajući zadatak:")
                        .reply_markup(create_calendar_keyboard(now.year(), now.month(), Some(now)))
                        .await?;
                    dialogue
                        .update(TaskState::AwaitingRecurringStartDate { task_name })
                        .await?;
                }
            }
            s if s.starts_with(CALENDAR_CALLBACK_SELECT_PREFIX) => {
                let date_str = s.trim_start_matches(CALENDAR_CALLBACK_SELECT_PREFIX);
                if let Ok(date) = NaiveDate::parse_from_str(date_str, CALENDAR_DEFAULT_DATE_FORMAT)
                {
                    remove_keyboard_buttons(&bot, &q).await;
                    handle_date_selection(bot.clone(), chat_id, dialogue, state, date).await?;
                }
            }
            s if s.starts_with(TIME_SELECTION_CALLBACK_PREFIX) => {
                let time_str = s.trim_start_matches(TIME_SELECTION_CALLBACK_PREFIX);
                remove_keyboard_buttons(&bot, &q).await;
                handle_time_selection(bot.clone(), chat_id, dialogue, state, time_str, scheduler)
                    .await?;
            }
            s if s.starts_with(CALENDAR_CALLBACK_PREV_PREFIX)
                || s.starts_with(CALENDAR_CALLBACK_NEXT_PREFIX) =>
            {
                handle_calendar_navigation(bot.clone(), &q, s).await?;
            }
            s if s == TASK_TYPE_CANCEL_ID
                || s == CALENDAR_CALLBACK_CANCEL
                || s == TIME_SELECTION_CANCEL =>
            {
                remove_keyboard_buttons(&bot, &q).await;
                bot.send_message(chat_id, "Zakazivanje zadatka je otkazano.")
                    .await?;
                dialogue.exit().await?;
            }
            s if s.starts_with(TIME_SELECTION_PAGE_PREFIX) => {
                let page: usize = s
                    .trim_start_matches(TIME_SELECTION_PAGE_PREFIX)
                    .parse()
                    .unwrap_or(0);

                if let Some(msg) = &q.message {
                    let min_time = {
                        let mut date_str: Option<String> = None;

                        match state {
                            TaskState::AwaitingSpecificTime { date, .. } => {
                                date_str = Some(date);
                            }
                            TaskState::AwaitingRecurringTime { end_date, .. } => {
                                date_str = Some(end_date);
                            }
                            _ => {}
                        }

                        if let Some(ds) = date_str {
                            let task_date =
                                NaiveDate::parse_from_str(&ds, CALENDAR_DEFAULT_DATE_FORMAT)
                                    .unwrap_or(get_current_date_in_bosnia());
                            if task_date == get_current_date_in_bosnia() {
                                Some(get_current_time_in_bosnia())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };

                    bot.edit_message_reply_markup(msg.chat().id, msg.id())
                        .reply_markup(create_time_selection_keyboard(min_time, page))
                        .await?;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

async fn receive_specific_date_text(
    _bot: Bot,
    _msg: Message,
    _dialogue: TaskDialogue,
    _task_name: String,
) -> ChatHandlerResult {
    // Fallback for text input instead of calendar
    Ok(())
}

async fn receive_recurring_start_text(
    _bot: Bot,
    _msg: Message,
    _dialogue: TaskDialogue,
    _task_name: String,
) -> ChatHandlerResult {
    // Fallback for text input
    Ok(())
}

async fn handle_date_selection(
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
        TaskState::AwaitingRecurringStartDate { task_name } => {
            let now = get_current_date_in_bosnia();
            let start_date = date.format(CALENDAR_DEFAULT_DATE_FORMAT).to_string();

            bot.send_message(chat_id, "Odaberi završni datum za ponavljajući zadatak:")
                .reply_markup(create_calendar_keyboard(now.year(), now.month(), Some(now)))
                .await?;
            dialogue
                .update(TaskState::AwaitingRecurringEndDate {
                    task_name,
                    start_date,
                })
                .await?;
        }
        TaskState::AwaitingRecurringEndDate {
            task_name,
            start_date,
        } => {
            bot.send_message(chat_id, "Odaberi vrijeme za zadatak:")
                .reply_markup(create_time_selection_keyboard(min_time, 0))
                .await?;
            dialogue
                .update(TaskState::AwaitingRecurringTime {
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

async fn handle_time_selection(
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

async fn handle_calendar_navigation(bot: Bot, q: &CallbackQuery, data: &str) -> ChatHandlerResult {
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

async fn remove_keyboard_buttons(bot: &Bot, q: &CallbackQuery) {
    if let Some(msg) = &q.message {
        bot.edit_message_reply_markup(msg.chat().id, msg.id())
            .await
            .ok();
    }
}

fn get_current_date_in_bosnia() -> NaiveDate {
    use chrono_tz::Europe::Sarajevo;
    let now_in_tz = Sarajevo.from_utc_datetime(&chrono::Utc::now().naive_utc());
    now_in_tz.date_naive()
}

fn get_current_time_in_bosnia() -> chrono::NaiveTime {
    use chrono_tz::Europe::Sarajevo;
    let now_in_tz = Sarajevo.from_utc_datetime(&chrono::Utc::now().naive_utc());
    now_in_tz.time()
}
