use chrono::{NaiveDate, TimeZone};
use teloxide::{
    Bot,
    payloads::SendMessageSetters,
    prelude::Requester,
    types::{BotName, ChatId, InlineKeyboardButton, InlineKeyboardMarkup, Message, ParseMode},
    utils::markdown,
};

pub async fn send_chat_message(bot: &Bot, chat_id: ChatId, text: String) {
    if let Err(err) = bot.send_message(chat_id, &text).await {
        log::error!("Failed to send message: {}", err);
    }
}

pub async fn send_chat_message_markdown(bot: &Bot, chat_id: ChatId, text: String) {
    if let Err(err) = bot
        .send_message(chat_id, &text)
        .parse_mode(ParseMode::MarkdownV2)
        .await
    {
        log::error!("Failed to send message: {}", err);
    }
}

pub async fn send_welcome_message(bot: Bot, chat_id: ChatId) {
    let bot_name = bot.get_my_name().await.unwrap_or(BotName {
        name: "Brat Sekretarko".to_string(),
    });

    send_chat_message(
            &bot,
            chat_id,
            format!("Es-selamu alejkum!\n\nJa sam {}. Ja sam tu da obavljam razne zadatke za vas.\nKoristite /help za listu komandi.", bot_name.name),
        )
        .await;
}

pub type ChatHandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub static CALENDAR_DEFAULT_DATE_FORMAT: &str = "%d.%m.%Y";

pub static TIME_DEFAULT_FORMAT: &str = "%H:%M";

pub fn get_user_mention(msg: &Message) -> Option<String> {
    if let Some(user) = &msg.from {
        if let Some(uname) = &user.username {
            return Some(format!("@{}", markdown::escape(uname)));
        } else {
            let display_name = markdown::escape(&user.first_name);
            return Some(format!("[{}](tg://user?id={})", display_name, user.id));
        }
    }

    None
}

pub fn get_current_date_in_bosnia() -> NaiveDate {
    use chrono_tz::Europe::Sarajevo;
    let now_in_tz = Sarajevo.from_utc_datetime(&chrono::Utc::now().naive_utc());
    now_in_tz.date_naive()
}

pub fn get_current_time_in_bosnia() -> chrono::NaiveTime {
    use chrono_tz::Europe::Sarajevo;
    let now_in_tz = Sarajevo.from_utc_datetime(&chrono::Utc::now().naive_utc());
    now_in_tz.time()
}

pub static TASK_TYPE_SPECIFIC_TEXT: &str = "određeni";
pub static TASK_TYPE_RECURRING_TEXT: &str = "ponavljajući";

pub static TASK_TYPE_SPECIFIC_ID: &str = "task_type_specific";
pub static TASK_TYPE_RECURRING_ID: &str = "task_type_recurring";
pub static TASK_TYPE_CANCEL_ID: &str = "task_cancel";

pub fn create_task_type_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback(
                format!("📅 {}", TASK_TYPE_SPECIFIC_TEXT),
                TASK_TYPE_SPECIFIC_ID,
            ),
            InlineKeyboardButton::callback(
                format!("🔄 {}", TASK_TYPE_RECURRING_TEXT),
                TASK_TYPE_RECURRING_ID,
            ),
        ],
        vec![InlineKeyboardButton::callback(
            "❌ Odustani",
            TASK_TYPE_CANCEL_ID,
        )],
    ])
}
