use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use lazy_static::lazy_static;
use regex::Regex;

use super::Event;

lazy_static! {
    pub static ref LOG_FILE_NAME_PATTERN: Regex = Regex::new(r"^output_log_.*\.txt$").unwrap();
    static ref LOG_HEADER_PATTERN: Regex = Regex::new(
        r"(?P<timestamp>\d{4}.\d{2}.\d{2} \d{2}:\d{2}:\d{2}) (?P<level>[^ ]+) *-  (?P<body>.*)"
    )
    .unwrap();
    static ref USER_AUTHENTICATED_PATTERN: Regex =
        Regex::new(r"\[Behaviour\] User Authenticated: (?P<username>\S+)").unwrap();
    static ref ON_JOINED_ROOM_PATTERN: Regex =
        Regex::new(r"\[Behaviour\] Finished entering world").unwrap();
    static ref ON_PLAYER_JOINED_PATTERN: Regex =
        Regex::new(r"\[Behaviour\] OnPlayerJoined (?P<username>.+?)($| \(usr_[a-z0-9-]+\))")
            .unwrap();
    static ref ON_LEFT_ROOM_PATTERN: Regex = Regex::new(r"\[Behaviour\] OnLeftRoom").unwrap();
    static ref ON_PLAYER_LEFT_PATTERN: Regex =
        Regex::new(r"\[Behaviour\] OnPlayerLeft (?P<username>.+?)($| \(usr_[a-z0-9-]+\))").unwrap();
}

#[cfg(target_os = "windows")]
pub fn get_log_dir_path() -> PathBuf {
    let mut path = PathBuf::from(env::var(OsString::from("AppData")).unwrap());
    path.pop();
    path.push("LocalLow");
    path.push("VRChat");
    path.push("vrchat");
    path
}

pub fn get_log_entries<P>(log_dir: P) -> io::Result<impl Iterator<Item = fs::DirEntry>>
where
    P: AsRef<Path>,
{
    let entries = fs::read_dir(log_dir)?;
    let log_files = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let file_type = entry.file_type().ok()?;
            if file_type.is_file() {
                let name = entry.file_name().into_string().ok()?;
                if LOG_FILE_NAME_PATTERN.is_match(&name) {
                    return Some(entry);
                }
            }
            None
        })
        .into_iter();
    Ok(log_files)
}

#[derive(Debug, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Log,
    Warning,
    Error,
}

#[derive(Debug, PartialEq)]
pub struct LogLine {
    pub time: DateTime<Local>,
    pub log_level: LogLevel,
    pub event: Option<Event>,
    pub body: String,
}

impl LogLine {
    pub fn from_line(line: &str) -> Option<LogLine> {
        let cap = LOG_HEADER_PATTERN.captures(line)?;
        let timestamp = cap.name("timestamp").unwrap().as_str();
        let timestamp = NaiveDateTime::parse_from_str(timestamp, "%Y.%m.%d %H:%M:%S").ok()?;
        let local_timestamp = Local.from_local_datetime(&timestamp).earliest()?;
        let level = match cap.name("level").unwrap().as_str() {
            "Debug" => LogLevel::Debug,
            "Log" => LogLevel::Log,
            "Warning" => LogLevel::Warning,
            "Error" => LogLevel::Error,
            _ => return None,
        };
        let body = cap.name("body").unwrap().as_str();
        let event = Self::parse_body(body);
        Some(LogLine {
            time: local_timestamp,
            log_level: level,
            body: body.to_owned(),
            event: event,
        })
    }

    fn parse_body(body: &str) -> Option<Event> {
        if ON_JOINED_ROOM_PATTERN.is_match(body) {
            return Some(Event::OnJoinedRoom);
        }

        if let Some(cap) = ON_PLAYER_JOINED_PATTERN.captures(body) {
            return Some(Event::OnPlayerJoined {
                user_name: cap.name("username").unwrap().as_str().to_owned(),
            });
        }

        if ON_LEFT_ROOM_PATTERN.is_match(body) {
            return Some(Event::OnLeftRoom);
        }

        if let Some(cap) = ON_PLAYER_LEFT_PATTERN.captures(body) {
            return Some(Event::OnPlayerLeft {
                user_name: cap.name("username").unwrap().as_str().to_owned(),
            });
        }

        if let Some(cap) = USER_AUTHENTICATED_PATTERN.captures(body) {
            return Some(Event::UserAuthenticated {
                user_name: cap.name("username").unwrap().as_str().to_owned(),
            });
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, NaiveDate};

    fn local_time(time: &NaiveDateTime) -> DateTime<Local> {
        Local.from_local_datetime(&time).latest().unwrap()
    }

    #[test]
    fn log_line_can_parse_on_joined_room_event() {
        let line = "2021.12.01 23:23:12 Log        -  [Behaviour] Finished entering world.";
        let actual = LogLine::from_line(line).expect("could not parse log line.");
        let expected = LogLine {
            time: local_time(&NaiveDate::from_ymd(2021, 12, 1).and_hms(23, 23, 12)),
            log_level: LogLevel::Log,
            event: Some(crate::vrc::Event::OnJoinedRoom),
            body: "[Behaviour] Finished entering world.".to_owned(),
        };
        assert_eq!(expected, actual);
    }

    #[test]
    fn log_line_can_parse_old_on_player_joined_event() {
        let line = "2021.12.01 23:23:13 Log        -  [Behaviour] OnPlayerJoined paralleltree";
        let actual = LogLine::from_line(line).expect("could not parse log line.");
        let expected = LogLine {
            time: local_time(&NaiveDate::from_ymd(2021, 12, 1).and_hms(23, 23, 13)),
            log_level: LogLevel::Log,
            event: Some(crate::vrc::Event::OnPlayerJoined {
                user_name: "paralleltree".to_owned(),
            }),
            body: "[Behaviour] OnPlayerJoined paralleltree".to_owned(),
        };
        assert_eq!(expected, actual);
    }
    #[test]
    fn log_line_can_parse_on_player_joined_event() {
        let line = "2021.12.01 23:23:13 Debug      -  [Behaviour] OnPlayerJoined paralleltree (usr_a58186d2-54f9-44c8-902b-6e03927f66c1)";
        let actual = LogLine::from_line(line).expect("could not parse log line.");
        let expected = LogLine {
            time: local_time(&NaiveDate::from_ymd(2021, 12, 1).and_hms(23, 23, 13)),
            log_level: LogLevel::Debug,
            event: Some(crate::vrc::Event::OnPlayerJoined {
                user_name: "paralleltree".to_owned(),
            }),
            body:
                "[Behaviour] OnPlayerJoined paralleltree (usr_a58186d2-54f9-44c8-902b-6e03927f66c1)"
                    .to_owned(),
        };
        assert_eq!(expected, actual);
    }

    #[test]
    fn log_line_can_parse_on_left_room_event() {
        let line = "2021.12.01 23:26:39 Log        -  [Behaviour] OnLeftRoom";
        let actual = LogLine::from_line(line).expect("could not parse log line.");
        let expected = LogLine {
            time: local_time(&NaiveDate::from_ymd(2021, 12, 1).and_hms(23, 26, 39)),
            log_level: LogLevel::Log,
            event: Some(crate::vrc::Event::OnLeftRoom),
            body: "[Behaviour] OnLeftRoom".to_owned(),
        };
        assert_eq!(expected, actual);
    }

    #[test]
    fn log_line_can_parse_old_on_player_left_event() {
        let line = "2021.12.01 23:26:39 Log        -  [Behaviour] OnPlayerLeft paralleltree";
        let actual = LogLine::from_line(line).expect("could not parse log line.");
        let expected = LogLine {
            time: local_time(&NaiveDate::from_ymd(2021, 12, 1).and_hms(23, 26, 39)),
            log_level: LogLevel::Log,
            event: Some(crate::vrc::Event::OnPlayerLeft {
                user_name: "paralleltree".to_owned(),
            }),
            body: "[Behaviour] OnPlayerLeft paralleltree".to_owned(),
        };
        assert_eq!(expected, actual);
    }
    #[test]
    fn log_line_can_parse_on_player_left_event() {
        let line = "2021.12.01 23:26:39 Debug      -  [Behaviour] OnPlayerLeft paralleltree (usr_a58186d2-54f9-44c8-902b-6e03927f66c1)";
        let actual = LogLine::from_line(line).expect("could not parse log line.");
        let expected = LogLine {
            time: local_time(&NaiveDate::from_ymd(2021, 12, 1).and_hms(23, 26, 39)),
            log_level: LogLevel::Debug,
            event: Some(crate::vrc::Event::OnPlayerLeft {
                user_name: "paralleltree".to_owned(),
            }),
            body:
                "[Behaviour] OnPlayerLeft paralleltree (usr_a58186d2-54f9-44c8-902b-6e03927f66c1)"
                    .to_owned(),
        };
        assert_eq!(expected, actual);
    }

    #[test]
    fn log_line_can_parse_user_authenticated_event() {
        let line = "2021.11.03 23:41:04 Log        -  [Behaviour] User Authenticated: paralleltree (usr_deadbeef-dead-beef-beef-deadbeefbeef)";
        let actual = LogLine::from_line(line).expect("could not parse log line.");
        let expected = LogLine {
            time: local_time(&NaiveDate::from_ymd(2021, 11, 3).and_hms(23, 41, 4)),
            log_level: LogLevel::Log,
            event: Some(crate::vrc::Event::UserAuthenticated {
                user_name: "paralleltree".to_owned(),
            }),
            body: "[Behaviour] User Authenticated: paralleltree (usr_deadbeef-dead-beef-beef-deadbeefbeef)".to_owned(),
        };
        assert_eq!(expected, actual);
    }
}
