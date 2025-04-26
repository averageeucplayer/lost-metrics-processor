use chrono::Duration;

pub const DB_VERSION: i32 = 5;
pub const SEVEN_DAYS_SECONDS: f32 = 604800.; 
pub const TIMEOUT_DELAY_MS: i64 = 1000;
pub const WORKSHOP_BUFF_ID: u32 = 9701;
pub const WINDOW_MS: i64 = 5_000;
pub const WINDOW_S: i64 = 5;
pub const API_URL: &str = "https://api.snow.xyz";
pub const LOW_PERFORMANCE_MODE_DURATION: Duration = Duration::milliseconds(1500);

pub const RAID_DIFFICULTIES: &[(&str, u32)] = &[
    ("Normal", 0),
    ("Hard", 1),
    ("Inferno", 2),
    ("Challenge", 3),
    ("Solo", 4),
    ("The First", 5),
];