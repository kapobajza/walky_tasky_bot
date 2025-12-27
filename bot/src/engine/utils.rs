use teloxide::{
    Bot,
    payloads::SendMessageSetters,
    prelude::Requester,
    types::{BotName, ChatId, Message, ParseMode},
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
