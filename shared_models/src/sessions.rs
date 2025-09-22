use chrono::{NaiveDateTime, Timelike};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Session {
    AS,  // 00:00 - 08:00
    LN,  // 08:00 - 15:00
    NYAM, // 15:00 - 18:00
    NYL, // 18:00 - 20:00
    NYPM, // 20:00 - 24:00
    None,
}

impl Session {
    pub fn as_str(&self) -> &'static str {
        match self {
            Session::AS => "AS",
            Session::LN => "LN",
            Session::NYAM => "NYAM",
            Session::NYL => "NYL",
            Session::NYPM => "NYPM",
            Session::None => "None",
        }
    }
}

pub fn session_from_timestamp_enum(ts: NaiveDateTime) -> Session {
    match ts.hour() {
        0..=7 => Session::AS,
        8..=14 => Session::LN,
        15..=17 => Session::NYAM,
        18..=19 => Session::NYL,
        20..=23 => Session::NYPM,
        _ => Session::None,
    }
}

impl FromStr for Session {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "AS" => Ok(Session::AS),
            "LN" => Ok(Session::LN),
            "NYAM" => Ok(Session::NYAM),
            "NYL" => Ok(Session::NYL),
            "NYPM" => Ok(Session::NYPM),
            _ => Err(()),
        }
    }
}