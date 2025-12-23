use teloxide::{
    Bot,
    payloads::SendMessageSetters,
    prelude::Requester,
    types::{BotName, ChatId, ParseMode},
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
