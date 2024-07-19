use chrono::{DateTime, Datelike, Weekday};

pub fn to_ja_datetime(datetime: DateTime<chrono::Utc>) -> String {
    let weekday = datetime.weekday();
    datetime
        .format("%Y/%m/%d({weekday}) %H:%M:%S.%3f")
        .to_string()
        .replace("{weekday}", convert_weekday_to_ja(weekday))
}

pub fn convert_weekday_to_ja(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "月",
        Weekday::Tue => "火",
        Weekday::Wed => "水",
        Weekday::Thu => "木",
        Weekday::Fri => "金",
        Weekday::Sat => "土",
        Weekday::Sun => "日",
    }
}
