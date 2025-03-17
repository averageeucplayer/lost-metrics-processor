use std::time::Duration;

pub const DB_VERSION: i32 = 5;
pub const TIMEOUT_DELAY_MS: i64 = 1000;
pub const WORKSHOP_BUFF_ID: u32 = 9701;
pub const WINDOW_MS: i64 = 5_000;
pub const WINDOW_S: i64 = 5;
pub const API_URL: &str = "https://api.snow.xyz";
pub const LOW_PERFORMANCE_MODE_DURATION: Duration = Duration::from_millis(1500);