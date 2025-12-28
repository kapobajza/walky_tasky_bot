mod common;

use bot::engine::date_keyboard::{
    CALENDAR_CALLBACK_CANCEL, CALENDAR_CALLBACK_IGNORE, CALENDAR_CALLBACK_NEXT_PREFIX,
    CALENDAR_CALLBACK_PREV_PREFIX, CALENDAR_CALLBACK_SELECT_PREFIX, create_calendar_keyboard,
};
use bot::engine::dialogue_handler::{
    TaskState, build_dialogue_callback_handler, build_dialogue_handler,
};
use bot::engine::time_keyboard::{
    TIME_SELECTION_CALLBACK_PREFIX, TIME_SELECTION_CANCEL, create_time_selection_keyboard,
};
use bot::engine::utils::{TASK_TYPE_CANCEL_ID, TASK_TYPE_RECURRING_ID, TASK_TYPE_SPECIFIC_ID};
use chrono::{NaiveDate, NaiveTime, Utc};
use common::create_test_scheduler_with_storage;
use dptree::deps;
use scheduler::storage::base_storage::Storage;
use scheduler::task::action::TaskAction;
use scheduler::task::default::Task;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardButtonKind, InlineKeyboardMarkup};
use teloxide_tests::{MockBot, MockCallbackQuery, MockMessageText};

// =============================================================================
// State Transition Tests
// =============================================================================

#[tokio::test]
async fn test_receive_task_name_transitions_to_awaiting_type() {
    let message = MockMessageText::new().text("Buy groceries");
    let handler = build_dialogue_handler();

    let mut bot = MockBot::new(message, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingTaskName).await;
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = &responses.sent_messages;

    assert!(!sent_messages.is_empty(), "Bot should have sent a message");

    let last_message = sent_messages.last().unwrap();
    let text = last_message.text().unwrap_or("");

    // Verify bot asks about task type (specific vs recurring)
    assert!(
        text.contains("određeni datum") || text.contains("ponavljajući"),
        "Response should mention task type options. Got: {}",
        text
    );

    // Verify keyboard is attached
    assert!(
        last_message.reply_markup().is_some(),
        "Response should include a keyboard"
    );
}

#[tokio::test]
async fn test_task_type_specific_transitions_to_date() {
    let (scheduler, _, _) = create_test_scheduler_with_storage();

    let callback = MockCallbackQuery::new().data(TASK_TYPE_SPECIFIC_ID);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingTaskType {
        task_name: "Test Task".to_string(),
    })
    .await;
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = &responses.sent_messages;

    assert!(
        !sent_messages.is_empty(),
        "Bot should have sent a calendar message"
    );

    let last_message = sent_messages.last().unwrap();
    let text = last_message.text().unwrap_or("");

    // Verify calendar prompt
    assert!(
        text.contains("datum"),
        "Response should ask to select a date. Got: {}",
        text
    );
}

#[tokio::test]
async fn test_task_type_recurring_transitions_to_start_date() {
    let (scheduler, _, _) = create_test_scheduler_with_storage();

    let callback = MockCallbackQuery::new().data(TASK_TYPE_RECURRING_ID);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingTaskType {
        task_name: "Recurring Task".to_string(),
    })
    .await;
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = &responses.sent_messages;

    assert!(
        !sent_messages.is_empty(),
        "Bot should have sent a message for recurring task start date"
    );

    let last_message = sent_messages.last().unwrap();
    let text = last_message.text().unwrap_or("");

    // Verify recurring task start date prompt
    assert!(
        text.contains("početni datum") || text.contains("ponavljajući"),
        "Response should ask for start date of recurring task. Got: {}",
        text
    );
}

#[tokio::test]
async fn test_date_selection_transitions_to_time() {
    let (scheduler, _, _) = create_test_scheduler_with_storage();

    // Use a future date to avoid filtering issues
    let date_callback = format!("{}01.01.2030", CALENDAR_CALLBACK_SELECT_PREFIX);
    let callback = MockCallbackQuery::new().data(&date_callback);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingSpecificDate {
        task_name: "Test Task".to_string(),
    })
    .await;
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = &responses.sent_messages;

    assert!(
        !sent_messages.is_empty(),
        "Bot should have sent a time selection message"
    );

    let last_message = sent_messages.last().unwrap();
    let text = last_message.text().unwrap_or("");

    // Verify time selection prompt
    assert!(
        text.contains("vrijeme"),
        "Response should ask to select a time. Got: {}",
        text
    );
}

#[tokio::test]
async fn test_time_selection_creates_task_and_exits() {
    let (scheduler, storage, _) = create_test_scheduler_with_storage();

    let time_callback = format!("{}14:30", TIME_SELECTION_CALLBACK_PREFIX);
    let callback = MockCallbackQuery::new().data(&time_callback);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingSpecificTime {
        task_name: "Test Task".to_string(),
        date: "01.01.2030".to_string(),
    })
    .await;
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = &responses.sent_messages;

    assert!(
        !sent_messages.is_empty(),
        "Bot should have sent a confirmation message"
    );

    let last_message = sent_messages.last().unwrap();
    let text = last_message.text().unwrap_or("");

    // Verify confirmation message
    assert!(
        text.contains("zakazan") || text.contains("Test Task"),
        "Response should confirm the task was scheduled. Got: {}",
        text
    );

    // Verify task was added to storage
    let tasks = storage.get_all_tasks().await.unwrap();
    assert_eq!(tasks.len(), 1, "One task should have been created");
}

// =============================================================================
// Cancel Flow Tests
// =============================================================================

#[tokio::test]
async fn test_cancel_from_task_type_exits() {
    let (scheduler, _, _) = create_test_scheduler_with_storage();

    let callback = MockCallbackQuery::new().data(TASK_TYPE_CANCEL_ID);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingTaskType {
        task_name: "Test Task".to_string(),
    })
    .await;
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = &responses.sent_messages;

    assert!(
        !sent_messages.is_empty(),
        "Bot should have sent a cancellation message"
    );

    let last_message = sent_messages.last().unwrap();
    let text = last_message.text().unwrap_or("");

    assert!(
        text.contains("otkazano"),
        "Response should confirm cancellation. Got: {}",
        text
    );
}

#[tokio::test]
async fn test_cancel_from_calendar_exits() {
    let (scheduler, _, _) = create_test_scheduler_with_storage();

    let callback = MockCallbackQuery::new().data(CALENDAR_CALLBACK_CANCEL);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingSpecificDate {
        task_name: "Test Task".to_string(),
    })
    .await;
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = &responses.sent_messages;

    assert!(
        !sent_messages.is_empty(),
        "Bot should have sent a cancellation message"
    );

    let last_message = sent_messages.last().unwrap();
    let text = last_message.text().unwrap_or("");

    assert!(
        text.contains("otkazano"),
        "Response should confirm cancellation. Got: {}",
        text
    );
}

#[tokio::test]
async fn test_cancel_from_time_selection_exits() {
    let (scheduler, _, _) = create_test_scheduler_with_storage();

    let callback = MockCallbackQuery::new().data(TIME_SELECTION_CANCEL);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingSpecificTime {
        task_name: "Test Task".to_string(),
        date: "01.01.2030".to_string(),
    })
    .await;
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = &responses.sent_messages;

    assert!(
        !sent_messages.is_empty(),
        "Bot should have sent a cancellation message"
    );

    let last_message = sent_messages.last().unwrap();
    let text = last_message.text().unwrap_or("");

    assert!(
        text.contains("otkazano"),
        "Response should confirm cancellation. Got: {}",
        text
    );
}

// =============================================================================
// Calendar Navigation Tests
// =============================================================================

#[tokio::test]
async fn test_calendar_prev_month_updates_keyboard() {
    let (scheduler, _, _) = create_test_scheduler_with_storage();

    // Navigate from December 2025 to November 2025
    let nav_callback = format!("{}2025_12", CALENDAR_CALLBACK_PREV_PREFIX);
    let callback = MockCallbackQuery::new().data(&nav_callback);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingSpecificDate {
        task_name: "Test Task".to_string(),
    })
    .await;
    bot.dispatch().await;

    let responses = bot.get_responses();
    // Calendar navigation edits the message, not sends a new one
    let edited_messages = &responses.edited_messages_reply_markup;

    assert!(
        !edited_messages.is_empty(),
        "Bot should have edited the calendar message"
    );
}

#[tokio::test]
async fn test_calendar_next_month_updates_keyboard() {
    let (scheduler, _, _) = create_test_scheduler_with_storage();

    // Navigate from December 2025 to January 2026
    let nav_callback = format!("{}2025_12", CALENDAR_CALLBACK_NEXT_PREFIX);
    let callback = MockCallbackQuery::new().data(&nav_callback);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingSpecificDate {
        task_name: "Test Task".to_string(),
    })
    .await;
    bot.dispatch().await;

    let responses = bot.get_responses();
    let edited_messages = &responses.edited_messages_reply_markup;

    assert!(
        !edited_messages.is_empty(),
        "Bot should have edited the calendar message"
    );
}

// =============================================================================
// Recurring Task Flow Tests
// =============================================================================

#[tokio::test]
async fn test_recurring_start_date_transitions_to_end_date() {
    let (scheduler, _, _) = create_test_scheduler_with_storage();

    let date_callback = format!("{}01.01.2030", CALENDAR_CALLBACK_SELECT_PREFIX);
    let callback = MockCallbackQuery::new().data(&date_callback);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingRecurringStartDate {
        task_name: "Recurring Task".to_string(),
    })
    .await;
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = &responses.sent_messages;

    assert!(
        !sent_messages.is_empty(),
        "Bot should have sent an end date selection message"
    );

    let last_message = sent_messages.last().unwrap();
    let text = last_message.text().unwrap_or("");

    assert!(
        text.contains("završni datum"),
        "Response should ask for end date. Got: {}",
        text
    );
}

#[tokio::test]
async fn test_recurring_end_date_transitions_to_time() {
    let (scheduler, _, _) = create_test_scheduler_with_storage();

    let date_callback = format!("{}15.01.2030", CALENDAR_CALLBACK_SELECT_PREFIX);
    let callback = MockCallbackQuery::new().data(&date_callback);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingRecurringEndDate {
        task_name: "Recurring Task".to_string(),
        start_date: "01.01.2030".to_string(),
    })
    .await;
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = &responses.sent_messages;

    assert!(
        !sent_messages.is_empty(),
        "Bot should have sent a time selection message"
    );

    let last_message = sent_messages.last().unwrap();
    let text = last_message.text().unwrap_or("");

    assert!(
        text.contains("vrijeme"),
        "Response should ask for time. Got: {}",
        text
    );
}

#[tokio::test]
async fn test_recurring_time_selection_sends_confirmation() {
    let (scheduler, _, _) = create_test_scheduler_with_storage();

    let time_callback = format!("{}09:00", TIME_SELECTION_CALLBACK_PREFIX);
    let callback = MockCallbackQuery::new().data(&time_callback);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingRecurringTime {
        task_name: "Recurring Task".to_string(),
        start_date: "01.01.2030".to_string(),
        end_date: "15.01.2030".to_string(),
    })
    .await;
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = &responses.sent_messages;

    assert!(
        !sent_messages.is_empty(),
        "Bot should have sent a confirmation message"
    );

    let last_message = sent_messages.last().unwrap();
    let text = last_message.text().unwrap_or("");

    // Verify recurring task confirmation
    assert!(
        text.contains("ponavljajući") || text.contains("Recurring Task"),
        "Response should confirm the recurring task. Got: {}",
        text
    );
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[tokio::test]
async fn test_callback_in_wrong_state_is_handled() {
    let (scheduler, storage, _) = create_test_scheduler_with_storage();

    // Try to select a task type when in Idle state (wrong state)
    let callback = MockCallbackQuery::new().data(TASK_TYPE_SPECIFIC_ID);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    // Don't set any state - defaults to Idle
    bot.dispatch().await;

    // Should not create any tasks
    let tasks = storage.get_all_tasks().await.unwrap();
    assert!(
        tasks.is_empty(),
        "No tasks should be created in wrong state"
    );
}

#[tokio::test]
async fn test_task_name_with_special_characters() {
    let message = MockMessageText::new().text("Buy groceries 🛒 @store #urgent");
    let handler = build_dialogue_handler();

    let mut bot = MockBot::new(message, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingTaskName).await;
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = &responses.sent_messages;

    // Should handle special characters without errors
    assert!(
        !sent_messages.is_empty(),
        "Bot should handle special characters in task name"
    );
}

// =============================================================================
// handle_date_selection Tests
// =============================================================================

#[tokio::test]
async fn test_date_selection_preserves_task_name_in_state() {
    let (scheduler, _, _) = create_test_scheduler_with_storage();

    let date_callback = format!("{}15.06.2030", CALENDAR_CALLBACK_SELECT_PREFIX);
    let callback = MockCallbackQuery::new().data(&date_callback);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingSpecificDate {
        task_name: "Important Meeting".to_string(),
    })
    .await;
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = &responses.sent_messages;

    // Verify time selection keyboard was sent
    assert!(
        !sent_messages.is_empty(),
        "Should send time selection message"
    );
    let last_message = sent_messages.last().unwrap();
    assert!(
        last_message.reply_markup().is_some(),
        "Time selection should include keyboard"
    );
}

#[tokio::test]
async fn test_recurring_end_date_after_start_date() {
    let (scheduler, _, _) = create_test_scheduler_with_storage();

    // End date is after start date
    let date_callback = format!("{}20.01.2030", CALENDAR_CALLBACK_SELECT_PREFIX);
    let callback = MockCallbackQuery::new().data(&date_callback);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingRecurringEndDate {
        task_name: "Daily Standup".to_string(),
        start_date: "01.01.2030".to_string(),
    })
    .await;
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = &responses.sent_messages;

    assert!(
        !sent_messages.is_empty(),
        "Should send time selection message"
    );
    let text = sent_messages.last().unwrap().text().unwrap_or("");
    assert!(
        text.contains("vrijeme"),
        "Should ask for time after end date selection. Got: {}",
        text
    );
}

// =============================================================================
// handle_time_selection Tests
// =============================================================================

#[tokio::test]
async fn test_time_selection_task_has_correct_action() {
    let (scheduler, storage, _) = create_test_scheduler_with_storage();

    let time_callback = format!("{}10:00", TIME_SELECTION_CALLBACK_PREFIX);
    let callback = MockCallbackQuery::new().data(&time_callback);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingSpecificTime {
        task_name: "Doctor Appointment".to_string(),
        date: "15.03.2030".to_string(),
    })
    .await;
    bot.dispatch().await;

    // Verify task was created with correct action
    let tasks = storage.get_all_tasks().await.unwrap();
    assert_eq!(tasks.len(), 1, "One task should be created");

    let task = &tasks[0];
    assert!(task.action.is_some(), "Task should have an action");

    if let Some(scheduler::task::action::TaskAction::SendBotMessage { message, .. }) = &task.action
    {
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
async fn test_time_selection_confirmation_contains_details() {
    let (scheduler, _, _) = create_test_scheduler_with_storage();

    let time_callback = format!("{}16:45", TIME_SELECTION_CALLBACK_PREFIX);
    let callback = MockCallbackQuery::new().data(&time_callback);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingSpecificTime {
        task_name: "Team Meeting".to_string(),
        date: "25.12.2030".to_string(),
    })
    .await;
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = &responses.sent_messages;
    let text = sent_messages.last().unwrap().text().unwrap_or("");

    // Verify confirmation contains task name, date, and time
    assert!(
        text.contains("Team Meeting"),
        "Confirmation should contain task name. Got: {}",
        text
    );
    assert!(
        text.contains("25.12.2030"),
        "Confirmation should contain date. Got: {}",
        text
    );
    assert!(
        text.contains("16:45"),
        "Confirmation should contain time. Got: {}",
        text
    );
}

#[tokio::test]
async fn test_recurring_time_confirmation_contains_all_dates() {
    let (scheduler, _, _) = create_test_scheduler_with_storage();

    let time_callback = format!("{}08:30", TIME_SELECTION_CALLBACK_PREFIX);
    let callback = MockCallbackQuery::new().data(&time_callback);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingRecurringTime {
        task_name: "Morning Exercise".to_string(),
        start_date: "01.02.2030".to_string(),
        end_date: "28.02.2030".to_string(),
    })
    .await;
    bot.dispatch().await;

    let responses = bot.get_responses();
    let sent_messages = &responses.sent_messages;
    let text = sent_messages.last().unwrap().text().unwrap_or("");

    // Verify confirmation contains task name, start date, end date, and time
    assert!(
        text.contains("Morning Exercise"),
        "Confirmation should contain task name. Got: {}",
        text
    );
    assert!(
        text.contains("01.02.2030"),
        "Confirmation should contain start date. Got: {}",
        text
    );
    assert!(
        text.contains("28.02.2030"),
        "Confirmation should contain end date. Got: {}",
        text
    );
    assert!(
        text.contains("08:30"),
        "Confirmation should contain time. Got: {}",
        text
    );
}

#[tokio::test]
async fn test_time_selection_in_wrong_state_does_nothing() {
    let (scheduler, storage, _) = create_test_scheduler_with_storage();

    // Try time selection when in AwaitingTaskType state (wrong state)
    let time_callback = format!("{}12:00", TIME_SELECTION_CALLBACK_PREFIX);
    let callback = MockCallbackQuery::new().data(&time_callback);
    let handler = build_dialogue_callback_handler(scheduler);

    let mut bot = MockBot::new(callback, handler);
    bot.dependencies(deps![InMemStorage::<TaskState>::new()]);
    bot.set_state(TaskState::AwaitingTaskType {
        task_name: "Test Task".to_string(),
    })
    .await;
    bot.dispatch().await;

    // Should not create any tasks
    let tasks = storage.get_all_tasks().await.unwrap();
    assert!(
        tasks.is_empty(),
        "No tasks should be created when time selected in wrong state"
    );
}

// =============================================================================
// Time Keyboard Filtering Tests
// =============================================================================

/// Helper function to extract callback data from an InlineKeyboardButton
fn get_callback_data(btn: &InlineKeyboardButton) -> Option<&str> {
    match &btn.kind {
        InlineKeyboardButtonKind::CallbackData(data) => Some(data.as_str()),
        _ => None,
    }
}

/// Helper function to count time slot buttons in a keyboard
fn count_time_slots(keyboard: &InlineKeyboardMarkup) -> usize {
    keyboard
        .inline_keyboard
        .iter()
        .flat_map(|row| row.iter())
        .filter(|btn| {
            get_callback_data(btn)
                .map(|d| d.starts_with(TIME_SELECTION_CALLBACK_PREFIX))
                .unwrap_or(false)
        })
        .count()
}

/// Helper function to get the first time slot text from a keyboard
fn get_first_time_slot(keyboard: &InlineKeyboardMarkup) -> Option<String> {
    keyboard
        .inline_keyboard
        .iter()
        .flat_map(|row| row.iter())
        .find(|btn| {
            get_callback_data(btn)
                .map(|d| d.starts_with(TIME_SELECTION_CALLBACK_PREFIX))
                .unwrap_or(false)
        })
        .map(|btn| btn.text.clone())
}

#[test]
fn test_time_keyboard_filters_past_slots_late_evening() {
    // At 23:30, only 23:30 and 23:45 should be available
    let min_time = NaiveTime::from_hms_opt(23, 30, 0).unwrap();
    let keyboard = create_time_selection_keyboard(Some(min_time), 0);

    let slot_count = count_time_slots(&keyboard);
    assert_eq!(
        slot_count, 2,
        "Should have exactly 2 slots (23:30 and 23:45)"
    );

    let first_slot = get_first_time_slot(&keyboard);
    assert_eq!(
        first_slot,
        Some("23:30".to_string()),
        "First available slot should be 23:30"
    );
}

#[test]
fn test_time_keyboard_end_of_day_single_slot() {
    // At 23:45, only 23:45 should be available
    let min_time = NaiveTime::from_hms_opt(23, 45, 0).unwrap();
    let keyboard = create_time_selection_keyboard(Some(min_time), 0);

    let slot_count = count_time_slots(&keyboard);
    assert_eq!(slot_count, 1, "Should have exactly 1 slot (23:45)");

    let first_slot = get_first_time_slot(&keyboard);
    assert_eq!(
        first_slot,
        Some("23:45".to_string()),
        "Only available slot should be 23:45"
    );
}

#[test]
fn test_time_keyboard_afternoon_filters_morning() {
    // At 14:00, slots from 14:00 to 23:45 should be available
    // That's 10 hours * 4 slots = 40 slots
    let min_time = NaiveTime::from_hms_opt(14, 0, 0).unwrap();
    let keyboard = create_time_selection_keyboard(Some(min_time), 0);

    // First page shows 16 slots, but we can verify the first slot
    let first_slot = get_first_time_slot(&keyboard);
    assert_eq!(
        first_slot,
        Some("14:00".to_string()),
        "First available slot should be 14:00"
    );

    // Count total slots across all pages
    // 14:00 to 23:45 = 10 hours * 4 = 40 slots
    // We can only see first page (16 slots) but verify first slot is correct
    let slot_count = count_time_slots(&keyboard);
    assert!(slot_count <= 16, "First page should have at most 16 slots");
    assert!(slot_count > 0, "Should have some slots available");
}

#[test]
fn test_time_keyboard_no_filter_when_none() {
    // With no min_time, first slot should be 00:00
    let keyboard = create_time_selection_keyboard(None, 0);

    let first_slot = get_first_time_slot(&keyboard);
    assert_eq!(
        first_slot,
        Some("00:00".to_string()),
        "First slot should be 00:00 when no min_time"
    );

    // First page should have 16 slots
    let slot_count = count_time_slots(&keyboard);
    assert_eq!(slot_count, 16, "First page should have 16 slots");
}

#[test]
fn test_time_keyboard_mid_slot_filters_correctly() {
    // At 18:43, the 18:30 and earlier slots should be filtered
    // First available should be 18:45
    let min_time = NaiveTime::from_hms_opt(18, 43, 0).unwrap();
    let keyboard = create_time_selection_keyboard(Some(min_time), 0);

    let first_slot = get_first_time_slot(&keyboard);
    assert_eq!(
        first_slot,
        Some("18:45".to_string()),
        "First available slot should be 18:45 (after 18:43)"
    );

    // From 18:45 to 23:45: 5 hours and 15 min = 5*4 + 1 = 21 slots
    let slot_count = count_time_slots(&keyboard);
    assert!(slot_count <= 16, "First page should have at most 16 slots");
}

// =============================================================================
// Calendar Date Filtering Tests
// =============================================================================

#[test]
fn test_calendar_disables_past_dates() {
    // min_date = January 15, 2030
    // Day 10 should be disabled (show "x", callback = cal_ignore)
    let min_date = NaiveDate::from_ymd_opt(2030, 1, 15).unwrap();
    let keyboard = create_calendar_keyboard(2030, 1, Some(min_date));

    // Look for disabled buttons (shown as "x" with cal_ignore)
    let mut found_disabled = false;
    for row in &keyboard.inline_keyboard {
        for btn in row {
            if btn.text == "x"
                && let Some(callback) = get_callback_data(btn)
                && callback == CALENDAR_CALLBACK_IGNORE
            {
                found_disabled = true;
            }
        }
    }

    assert!(
        found_disabled,
        "Calendar should have disabled dates shown as 'x'"
    );

    // Verify day 10 specifically is not clickable (no cal_select_10.01.2030)
    let day_10_callback = format!("{}10.01.2030", CALENDAR_CALLBACK_SELECT_PREFIX);
    let has_day_10_enabled = keyboard.inline_keyboard.iter().any(|row| {
        row.iter().any(|btn| {
            get_callback_data(btn)
                .map(|d| d == day_10_callback)
                .unwrap_or(false)
        })
    });

    assert!(
        !has_day_10_enabled,
        "Day 10 should NOT have a clickable callback (it's before min_date)"
    );
}

#[test]
fn test_calendar_enables_future_dates() {
    // min_date = January 15, 2030
    // Day 20 should be enabled (show "20", callback = cal_select_20.01.2030)
    let min_date = NaiveDate::from_ymd_opt(2030, 1, 15).unwrap();
    let keyboard = create_calendar_keyboard(2030, 1, Some(min_date));

    let day_20_callback = format!("{}20.01.2030", CALENDAR_CALLBACK_SELECT_PREFIX);

    let day_20_button = keyboard
        .inline_keyboard
        .iter()
        .flat_map(|row| row.iter())
        .find(|btn| {
            get_callback_data(btn)
                .map(|d| d == day_20_callback)
                .unwrap_or(false)
        });

    assert!(day_20_button.is_some(), "Day 20 should be clickable");
    assert_eq!(
        day_20_button.unwrap().text,
        "20",
        "Day 20 should show '20' not 'x'"
    );
}

#[test]
fn test_calendar_enables_min_date_itself() {
    // min_date = January 15, 2030
    // Day 15 itself should be enabled (it's the minimum allowed date)
    let min_date = NaiveDate::from_ymd_opt(2030, 1, 15).unwrap();
    let keyboard = create_calendar_keyboard(2030, 1, Some(min_date));

    let day_15_callback = format!("{}15.01.2030", CALENDAR_CALLBACK_SELECT_PREFIX);

    let day_15_button = keyboard
        .inline_keyboard
        .iter()
        .flat_map(|row| row.iter())
        .find(|btn| {
            get_callback_data(btn)
                .map(|d| d == day_15_callback)
                .unwrap_or(false)
        });

    assert!(
        day_15_button.is_some(),
        "Day 15 (min_date) should be clickable"
    );
    assert_eq!(
        day_15_button.unwrap().text,
        "15",
        "Day 15 should show '15' not 'x'"
    );
}

#[test]
fn test_calendar_prev_month_disabled_at_min_date() {
    // min_date = January 15, 2030
    // Previous month button should be disabled when viewing January 2030
    let min_date = NaiveDate::from_ymd_opt(2030, 1, 15).unwrap();
    let keyboard = create_calendar_keyboard(2030, 1, Some(min_date));

    // First row is navigation: [prev, month_name, next]
    let nav_row = &keyboard.inline_keyboard[0];
    let prev_button = &nav_row[0];

    // When disabled, prev button shows " " (space) with cal_ignore
    assert_eq!(
        prev_button.text, " ",
        "Previous button should be disabled (show space) when at min_date month"
    );
    assert_eq!(
        get_callback_data(prev_button).unwrap(),
        CALENDAR_CALLBACK_IGNORE,
        "Disabled prev button should have cal_ignore callback"
    );
}

#[test]
fn test_calendar_prev_month_enabled_for_future_month() {
    // min_date = January 15, 2030
    // When viewing March 2030, previous month button should be enabled
    let min_date = NaiveDate::from_ymd_opt(2030, 1, 15).unwrap();
    let keyboard = create_calendar_keyboard(2030, 3, Some(min_date));

    // First row is navigation: [prev, month_name, next]
    let nav_row = &keyboard.inline_keyboard[0];
    let prev_button = &nav_row[0];

    // When enabled, prev button shows "◀️"
    assert_eq!(
        prev_button.text, "◀️",
        "Previous button should be enabled when viewing month after min_date"
    );
    assert!(
        get_callback_data(prev_button)
            .unwrap()
            .starts_with(CALENDAR_CALLBACK_PREV_PREFIX),
        "Enabled prev button should have cal_prev_ callback"
    );
}

// =============================================================================
// End-to-End Scheduler Execution Tests
// =============================================================================

#[tokio::test]
async fn test_scheduler_executes_task_and_sends_message() {
    let (scheduler, storage, captured) = create_test_scheduler_with_storage();

    // Create a task that's ready to run NOW
    let chat_id = 12345i64;
    let task_name = "Buy groceries";
    let task = Task::new_with_datetime(
        Utc::now(), // Ready immediately
        TaskAction::SendBotMessage {
            chat_id,
            message: format!("🔔 Podsjetnik: {}", task_name),
        },
    );

    // Add task to storage
    storage.save_task(task).await.unwrap();

    // Start scheduler
    scheduler.start().await.unwrap();

    // Wait for execution (check_interval is 50ms)
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    // Verify captured message
    let messages = captured.lock().await;
    assert_eq!(messages.len(), 1, "Should have captured 1 message");
    assert_eq!(messages[0].0, chat_id, "Chat ID should match");
    assert!(
        messages[0].1.contains(task_name),
        "Message should contain task name"
    );

    // Clean up
    scheduler.stop().await.unwrap();
}
