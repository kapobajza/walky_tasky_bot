use chrono::{Datelike, NaiveDate};
use scheduler::task::task_scheduler::TaskScheduler;
use teloxide::{
    Bot,
    dispatching::{DpHandlerDescription, HandlerExt, UpdateFilterExt, dialogue::InMemStorage},
    dptree::{self, Handler},
    payloads::{EditMessageReplyMarkupSetters, SendMessageSetters},
    prelude::{Dialogue, Requester},
    types::{CallbackQuery, Message, Update},
};

use crate::engine::{
    assigne_mention_handler::handle_assigne_mention_callback,
    date_keyboard::{
        CALENDAR_CALLBACK_CANCEL, CALENDAR_CALLBACK_NEXT_PREFIX, CALENDAR_CALLBACK_PREV_PREFIX,
        CALENDAR_CALLBACK_SELECT_PREFIX, create_calendar_keyboard,
        handle_keyboard_calendar_navigation, handle_keyboard_date_selection,
    },
    time_keyboard::{
        TIME_SELECTION_CALLBACK_PREFIX, TIME_SELECTION_CANCEL, TIME_SELECTION_PAGE_PREFIX,
        create_time_selection_keyboard, handle_keyboard_time_selection,
    },
    utils::{
        CALENDAR_DEFAULT_DATE_FORMAT, ChatHandlerResult, TASK_TYPE_CANCEL_ID,
        TASK_TYPE_RECURRING_ID, TASK_TYPE_SPECIFIC_ID, create_task_type_keyboard,
        get_current_date_in_bosnia, get_current_time_in_bosnia,
    },
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
    AwaitingRangeStartDate {
        task_name: String,
    },
    AwaitingRangeEndDate {
        task_name: String,
        start_date: String,
    },
    AwaitingRangeTime {
        task_name: String,
        start_date: String,
        end_date: String,
    },
    AwaitingAssigneeMention {
        task_name: String,
        date: String,
        end_date: Option<String>,
        time: String,
    },
}

pub type TaskDialogue = Dialogue<TaskState, InMemStorage<TaskState>>;

pub fn build_dialogue_handler(
    scheduler: TaskScheduler,
) -> Handler<
    'static,
    Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>,
    DpHandlerDescription,
> {
    Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<TaskState>, TaskState>()
        .branch(dptree::case![TaskState::AwaitingTaskName].endpoint(handle_task_name_callback))
        .branch(
            dptree::case![TaskState::AwaitingSpecificDate { task_name }]
                .endpoint(handle_specific_date_text_callback),
        )
        .branch(
            dptree::case![TaskState::AwaitingRangeStartDate { task_name }]
                .endpoint(handle_recurring_start_text_callback),
        )
        .branch(
            dptree::case![TaskState::AwaitingAssigneeMention {
                task_name,
                date,
                time,
                end_date,
            }]
            .endpoint(move |bot, msg, dialogue| {
                let scheduler = scheduler.clone();
                async move { handle_assigne_mention_callback(bot, msg, dialogue, &scheduler).await }
            }),
        )
}

pub fn build_dialogue_callback_handler() -> Handler<
    'static,
    Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>,
    DpHandlerDescription,
> {
    Update::filter_callback_query()
        .enter_dialogue::<CallbackQuery, InMemStorage<TaskState>, TaskState>()
        .endpoint(handle_dialogue_callback)
}

async fn handle_task_name_callback(
    bot: Bot,
    msg: Message,
    dialogue: TaskDialogue,
) -> ChatHandlerResult {
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

async fn handle_dialogue_callback(
    bot: Bot,
    q: CallbackQuery,
    dialogue: TaskDialogue,
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
                        .update(TaskState::AwaitingRangeStartDate { task_name })
                        .await?;
                }
            }
            s if s.starts_with(CALENDAR_CALLBACK_SELECT_PREFIX) => {
                let date_str = s.trim_start_matches(CALENDAR_CALLBACK_SELECT_PREFIX);
                if let Ok(date) = NaiveDate::parse_from_str(date_str, CALENDAR_DEFAULT_DATE_FORMAT)
                {
                    remove_keyboard_buttons(&bot, &q).await;
                    handle_keyboard_date_selection(bot.clone(), chat_id, dialogue, state, date)
                        .await?;
                }
            }
            s if s.starts_with(TIME_SELECTION_CALLBACK_PREFIX) => {
                let time_str = s.trim_start_matches(TIME_SELECTION_CALLBACK_PREFIX);
                remove_keyboard_buttons(&bot, &q).await;
                handle_keyboard_time_selection(bot.clone(), chat_id, dialogue, state, time_str)
                    .await?;
            }
            s if s.starts_with(CALENDAR_CALLBACK_PREV_PREFIX)
                || s.starts_with(CALENDAR_CALLBACK_NEXT_PREFIX) =>
            {
                handle_keyboard_calendar_navigation(bot.clone(), &q, s).await?;
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
                            TaskState::AwaitingRangeTime { end_date, .. } => {
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

async fn handle_specific_date_text_callback(
    _bot: Bot,
    _msg: Message,
    _dialogue: TaskDialogue,
    _task_name: String,
) -> ChatHandlerResult {
    // Fallback for text input instead of calendar
    Ok(())
}

async fn handle_recurring_start_text_callback(
    _bot: Bot,
    _msg: Message,
    _dialogue: TaskDialogue,
    _task_name: String,
) -> ChatHandlerResult {
    // Fallback for text input
    Ok(())
}

async fn remove_keyboard_buttons(bot: &Bot, q: &CallbackQuery) {
    if let Some(msg) = &q.message {
        bot.edit_message_reply_markup(msg.chat().id, msg.id())
            .await
            .ok();
    }
}
