use chrono::{prelude::*, NaiveDateTime};
use std::time::SystemTime;

#[inline]
pub fn timestamp() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => 0,
    }
}

/// 日期转换
/// 传入格式必须  YYYY-MM-DD HH:mm::ss
/// # Examples
/// ```
/// from_str("1970-01-01 00:00:00")
/// ```
#[inline]
pub fn from_str(datetime_str: &str) -> DateTime<Local> {
    let date_time_arr = datetime_str.split(" ").collect::<Vec<&str>>();
    let y_m_d = date_time_arr[0].split("-").collect::<Vec<&str>>();
    let h_m_s = date_time_arr[1].split(":").collect::<Vec<&str>>();
    let year = if let Ok(v) = y_m_d[0].parse::<i32>() {
        v
    } else {
        1970
    };
    let month = if let Ok(v) = y_m_d[1].parse::<u32>() {
        v
    } else {
        1
    };
    let day = if let Ok(v) = y_m_d[2].parse::<u32>() {
        v
    } else {
        1
    };
    let hour = if let Ok(v) = h_m_s[0].parse::<u32>() {
        v
    } else {
        0
    };
    let minute = if let Ok(v) = h_m_s[1].parse::<u32>() {
        v
    } else {
        0
    };
    let second = if let Ok(v) = h_m_s[2].parse::<u32>() {
        v
    } else {
        0
    };
    Local.ymd(year, month, day).and_hms(hour, minute, second)
}

/// 获取当前时间字符串
#[inline]
pub fn to_string() -> String {
    let local: DateTime<Local> = Local::now();
    local.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 格式化时间
#[inline]
pub fn format(format_str: &str) -> String {
    let local: DateTime<Local> = Local::now();
    local.format(format_str).to_string()
}

/// 获取当前日期
#[inline]
pub fn now() -> DateTime<Local> {
    Local::now()
}

/// 获取时分秒
#[inline]
pub fn get_hms() -> (u32, u32, u32) {
    let now = now();
    (now.hour(), now.minute(), now.second())
}

/// 获取年月日
#[inline]
pub fn get_ymd() -> (u32, u32, u32) {
    let now = now();
    (now.year() as u32, now.month(), now.day())
}

/// 时间戳转日期时间格式
#[inline]
pub fn date_to_str(timestamp: i64) -> String {
    let dt = NaiveDateTime::from_timestamp(timestamp, 0);
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}
