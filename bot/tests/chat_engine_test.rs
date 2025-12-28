mod common;

use bot::engine::{
    bot_mentioned_handler::{build_bot_mentioned_handler, is_bot_mentioned},
    command_handler::build_command_handler,
    dialogue_handler::{TaskState, build_dialogue_handler},
    utils::get_current_user_mention,
};
use common::create_test_scheduler_with_storage;
use dptree::deps;
use scheduler::{storage::base_storage::Storage, task::action::TaskAction};
use teloxide::{
    dispatching::dialogue::InMemStorage,
    types::{MessageEntity, MessageEntityKind, User, UserId},
};
use teloxide_tests::{MockBot, MockMessageText};

fn create_mention_entity(offset: usize, length: usize) -> MessageEntity {
    MessageEntity {
        offset,
        length,
        kind: MessageEntityKind::Mention,
    }
}

#[test]
fn test_is_bot_mentioned_exact_match() {
    let msg = MockMessageText::new()
        .text("@mybot")
        .entities(vec![create_mention_entity(0, 6)])
        .build();

    assert!(is_bot_mentioned(&msg, "mybot"));
}

#[test]
fn test_is_bot_mentioned_in_sentence() {
    let msg = MockMessageText::new()
        .text("Hello @mybot, how are you?")
        .entities(vec![create_mention_entity(6, 6)])
        .build();

    assert!(is_bot_mentioned(&msg, "mybot"));
}

#[test]
fn test_is_bot_mentioned_wrong_username() {
    let msg = MockMessageText::new()
        .text("@user")
        .entities(vec![create_mention_entity(0, 5)])
        .build();

    assert!(!is_bot_mentioned(&msg, "mybot"));
}

#[test]
fn test_is_bot_mentioned_no_entities() {
    let msg = MockMessageText::new().text("Hello mybot").build();
    assert!(!is_bot_mentioned(&msg, "mybot"));
}

#[test]
fn test_is_bot_mentioned_multiple_entities() {
    let msg = MockMessageText::new()
        .text("Hello @user and @mybot")
        .entities(vec![
            create_mention_entity(6, 5),
            create_mention_entity(16, 6),
        ])
        .build();

    assert!(is_bot_mentioned(&msg, "mybot"));
}

#[test]
fn test_get_user_mention_with_username() {
    let msg = MockMessageText::new()
        .text("Hello @user")
        .from(User {
            id: UserId(12345),
            is_bot: false,
            first_name: "Test".to_string(),
            last_name: None,
            username: Some("user".to_string()),
            language_code: Some("en".to_string()),
            is_premium: false,
            added_to_attachment_menu: false,
        })
        .entities(vec![create_mention_entity(6, 5)])
        .build();

    assert!(get_current_user_mention(&msg) == Some(format!("@{}", "user")));
}

#[test]
fn test_get_user_mention_without_username() {
    let user_id = 67890;
    let first_name = "John";
    let msg = MockMessageText::new()
        .text("Hello John")
        .from(User {
            id: UserId(user_id),
            is_bot: false,
            first_name: first_name.to_string(),
            last_name: None,
            username: None,
            language_code: Some("en".to_string()),
            is_premium: false,
            added_to_attachment_menu: false,
        })
        .build();

    assert!(
        get_current_user_mention(&msg)
            == Some(format!("[{}](tg://user?id={})", first_name, user_id))
    );
}

#[test]
fn test_get_user_mention_escapes_special_characters() {
    let user_id = 11223;
    let first_name = "John_Doe*";
    let msg = MockMessageText::new()
        .text("Hello John_Doe*")
        .from(User {
            id: UserId(user_id),
            is_bot: false,
            first_name: first_name.to_string(),
            last_name: None,
            username: None,
            language_code: Some("en".to_string()),
            is_premium: false,
            added_to_attachment_menu: false,
        })
        .build();

    assert!(
        get_current_user_mention(&msg)
            == Some(format!("[{}](tg://user?id={})", "John\\_Doe\\*", user_id))
    );
}

#[tokio::test]
async fn test_help_command_shows_command_list() {
    let message = MockMessageText::new().text("/help");
    let handler = build_command_handler();

    let mut bot = MockBot::new(message, handler);

    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = responses.sent_messages;

    assert!(!sent_messages.is_empty());

    let last_message_text = sent_messages.last().unwrap().text().unwrap();
    assert!(
        last_message_text.contains("Evo šta mogu da radim:"),
        "Help command response does not contain expected title"
    );
    assert!(
        last_message_text.contains("/help"),
        "Help command response does not contain /help command"
    );
    assert!(
        last_message_text.contains("/novi_zadatak"),
        "Help command response does not contain /novi_zadatak command"
    );
}

#[tokio::test]
async fn test_start_command_sends_welcome_message() {
    let message = MockMessageText::new().text("/start");
    let handler = build_command_handler();

    let mut bot = MockBot::new(message, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = responses.sent_messages;

    assert!(!sent_messages.is_empty());

    let last_message_text = sent_messages.last().unwrap().text().unwrap();

    assert!(
        last_message_text.contains("Es-selamu alejkum"),
        "Start command response does not contain welcome message"
    );
    assert!(
        last_message_text.contains("Brat Sekretarko"),
        "Start command response does not contain bot name"
    );
}

#[tokio::test]
async fn test_novi_zadatak_command_prompts_for_task_name() {
    let message = MockMessageText::new().text("/novi_zadatak");
    let handler = build_command_handler();

    let mut bot = MockBot::new(message, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = responses.sent_messages;

    assert!(!sent_messages.is_empty());

    let last_message_text = sent_messages.last().unwrap().text().unwrap();
    assert!(
        last_message_text.contains("Unesite naziv zadatka koji želite da kreirate:"),
        "NoviZadatak command response does not prompt for task name"
    );
}

#[tokio::test]
async fn test_bot_mention_responds_with_command_list() {
    let message = MockMessageText::new()
        .from(User {
            id: UserId(54321),
            is_bot: false,
            first_name: "Jašar".to_string(),
            last_name: Some("Ahmedovski".to_string()),
            username: Some("jašar".to_string()),
            language_code: Some("en".to_string()),
            is_premium: false,
            added_to_attachment_menu: false,
        })
        .text("Hey @mybot, what can you do?")
        .entities(vec![create_mention_entity(4, 6)]);
    let handler = build_bot_mentioned_handler("mybot".to_string());

    let mut bot = MockBot::new(message, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = responses.sent_messages;

    assert!(!sent_messages.is_empty());

    let last_message_text = sent_messages.last().unwrap().text().unwrap();
    assert!(
        last_message_text.contains("@jašar"),
        "Bot mention response does not contain expected title"
    );
    assert!(
        last_message_text.contains("/help"),
        "Bot mention response does not contain /help command"
    );
    assert!(
        last_message_text.contains("/novi\\_zadatak"),
        "Bot mention response does not contain /novi_zadatak command"
    );
}

#[tokio::test]
async fn test_assigne_mention_task_has_correct_action() {
    let (scheduler, storage, _) = create_test_scheduler_with_storage();

    let message = MockMessageText::new()
        .text("Hello there @user")
        .entities(vec![create_mention_entity(12, 5)]);
    let handler = build_dialogue_handler(scheduler);

    let mut bot = MockBot::new(message, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingAssigneeMention {
        task_name: "Doctor Appointment".to_string(),
        date: "15.03.2030".to_string(),
        time: "10:00".to_string(),
    })
    .await;
    bot.dispatch().await;

    // Verify task was created with correct action
    let tasks = storage.get_all_tasks().await.unwrap();
    assert_eq!(tasks.len(), 1, "One task should be created");

    let task = &tasks[0];
    assert!(task.action.is_some(), "Task should have an action");

    if let Some(TaskAction::SendBotMessage { message, .. }) = &task.action {
        assert!(
            message.contains("Doctor Appointment"),
            "Task message should contain task name. Got: {}",
            message
        );
        assert!(
            message.contains("Podsjetnik"),
            "Task message should be a reminder. Got: {}",
            message
        );
    } else {
        panic!("Task action should be SendBotMessage");
    }
}

#[tokio::test]
async fn test_assignee_mention_with_tg_link_has_correct_action() {
    let (scheduler, storage, _) = create_test_scheduler_with_storage();

    let message = MockMessageText::new()
        .text("Hello there [User](tg://user?id=12345)")
        .entities(vec![MessageEntity {
            offset: 12,
            length: 26,
            kind: MessageEntityKind::TextMention {
                user: User {
                    id: UserId(12345),
                    is_bot: false,
                    first_name: "User".to_string(),
                    last_name: None,
                    username: None,
                    language_code: Some("en".to_string()),
                    is_premium: false,
                    added_to_attachment_menu: false,
                },
            },
        }]);
    let handler = build_dialogue_handler(scheduler);

    let mut bot = MockBot::new(message, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingAssigneeMention {
        task_name: "Meeting".to_string(),
        date: "20.04.2030".to_string(),
        time: "15:30".to_string(),
    })
    .await;
    bot.dispatch().await;

    // Verify task was created with correct action
    let tasks = storage.get_all_tasks().await.unwrap();
    assert_eq!(tasks.len(), 1, "One task should be created");

    let task = &tasks[0];
    assert!(task.action.is_some(), "Task should have an action");

    if let Some(TaskAction::SendBotMessage { message, .. }) = &task.action {
        assert!(
            message.contains("Meeting"),
            "Task message should contain task name. Got: {}",
            message
        );
        assert!(
            message.contains("Podsjetnik"),
            "Task message should be a reminder. Got: {}",
            message
        );
    } else {
        panic!("Task action should be SendBotMessage");
    }
}

#[tokio::test]
async fn test_assignee_mention_with_tg_handles_special_chars() {
    let (scheduler, storage, _) = create_test_scheduler_with_storage();

    let message = MockMessageText::new()
        .text("Hello there [John_Doe*](tg://user?id=67890)")
        .entities(vec![MessageEntity {
            offset: 12,
            length: 31,
            kind: MessageEntityKind::TextMention {
                user: User {
                    id: UserId(67890),
                    is_bot: false,
                    first_name: "John_Doe*".to_string(),
                    last_name: None,
                    username: None,
                    language_code: Some("en".to_string()),
                    is_premium: false,
                    added_to_attachment_menu: false,
                },
            },
        }]);
    let handler = build_dialogue_handler(scheduler);

    let mut bot = MockBot::new(message, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingAssigneeMention {
        task_name: "Code.Review".to_string(),
        date: "25.05.2030".to_string(),
        time: "09:00".to_string(),
    })
    .await;
    bot.dispatch().await;

    // Verify task was created with correct action
    let tasks = storage.get_all_tasks().await.unwrap();
    assert_eq!(tasks.len(), 1, "One task should be created");

    let task = &tasks[0];
    assert!(task.action.is_some(), "Task should have an action");

    if let Some(TaskAction::SendBotMessage { message, .. }) = &task.action {
        assert!(
            message.contains("Code.Review"),
            "Task message should contain task name. Got: {}",
            message
        );
        assert!(
            message.contains("Podsjetnik"),
            "Task message should be a reminder. Got: {}",
            message
        );
    } else {
        panic!("Task action should be SendBotMessage");
    }
}
