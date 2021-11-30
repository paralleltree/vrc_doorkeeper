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
    pub static ref LOG_FILE_NAME_PATTERN: Regex =
        Regex::new(r"^output_log_\d{2}-\d{2}-\d{2}\.txt$").unwrap();
    static ref LOG_HEADER_PATTERN: Regex = Regex::new(
        r"(?P<timestamp>\d{4}.\d{2}.\d{2} \d{2}:\d{2}:\d{2}) (?P<level>[^ ]+) *-  (?P<body>.*)"
    )
    .unwrap();
    static ref USER_AUTHENTICATED_PATTERN: Regex =
        Regex::new(r"User Authenticated: (?P<username>.+)").unwrap();
    static ref ON_JOINED_ROOM_PATTERN: Regex = Regex::new("OnJoinedRoom").unwrap();
    static ref ON_PLAYER_JOINED_PATTERN: Regex =
        Regex::new(r"OnPlayerJoined (?P<username>.+)").unwrap();
    static ref ON_LEFT_ROOM_PATTERN: Regex = Regex::new("OnLeftRoom").unwrap();
    static ref ON_PLAYER_LEFT_PATTERN: Regex =
        Regex::new(r"OnPlayerLeft (?P<username>.+)").unwrap();
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
