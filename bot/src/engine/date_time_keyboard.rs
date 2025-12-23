use chrono::{Datelike, NaiveDate, NaiveTime, Timelike};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

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

pub static TIME_SELECTION_CALLBACK_PREFIX: &str = "time_select_";
pub static TIME_SELECTION_CANCEL: &str = "time_cancel";
pub static TIME_SELECTION_IGNORE: &str = "time_ignore";
pub static TIME_SELECTION_PAGE_PREFIX: &str = "time_page_";

pub fn create_time_selection_keyboard(
    min_time: Option<NaiveTime>,
    page: usize, // new parameter
) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = vec![];
    let min_minutes = min_time.map(|d| d.hour() * 60 + d.minute());

    let slots_per_page = 16;
    let total_slots = 24 * 4;
    let start_slot = page * slots_per_page;
    let end_slot = ((page + 1) * slots_per_page).min(total_slots);

    for slot in start_slot..end_slot {
        let hour = slot / 4;
        let minute = (slot % 4) * 15;
        let total_minutes = (hour * 60 + minute) as u32;
        let is_disabled = min_minutes.map(|m| total_minutes < m).unwrap_or(false);

        let button = InlineKeyboardButton::callback(
            if is_disabled {
                "x".to_string()
            } else {
                format!("{:02}:{:02}", hour, minute)
            },
            if is_disabled {
                TIME_SELECTION_IGNORE.to_string()
            } else {
                format!(
                    "{}{:02}:{:02}",
                    TIME_SELECTION_CALLBACK_PREFIX, hour, minute
                )
            },
        );

        if (slot - start_slot).is_multiple_of(4) {
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
    if end_slot < total_slots {
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
