use std::fs;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

use crate::vrc::log::LogLine;

#[derive(Debug)]
struct ContinuousFileReader {
    file_path: PathBuf,
    read_bytes: u64,
}

impl ContinuousFileReader {
    fn new(file_path: PathBuf) -> Self {
        ContinuousFileReader {
            file_path,
            read_bytes: 0,
        }
    }

    fn read_appended_lines<F>(&mut self, mut callback: F) -> io::Result<()>
    where
        F: FnMut(&str),
    {
        let mut f = File::open(&self.file_path)?;
        f.seek(SeekFrom::Start(self.read_bytes))?;
        let mut reader = BufReader::new(f);
        let mut buf = String::new();
        loop {
            let len = reader.read_line(&mut buf)?;
            if len == 0 {
                break;
            }
            self.read_bytes += len as u64;
            callback(buf.trim_end());
            buf.clear();
        }
        Ok(())
    }
}

pub trait LogLineProcessor {
    fn process_line(&mut self, line: LogLine, is_first: bool);
}

pub struct VrChatLogProcessor<'a, T: LogLineProcessor> {
    log_dir: PathBuf,
    processor: &'a mut T,
    reader: Option<ContinuousFileReader>,
}

impl<T: LogLineProcessor> VrChatLogProcessor<'_, T> {
    pub fn new(log_dir: PathBuf, processor: &mut T) -> VrChatLogProcessor<T> {
        VrChatLogProcessor {
            log_dir,
            processor,
            reader: None,
        }
    }

    pub fn process_log(&mut self) -> io::Result<()> {
        let log_entries = crate::vrc::log::get_log_entries(&self.log_dir)?;
        let latest_log_path = match find_latest_log_path(log_entries) {
            Some(path) => path,
            None => return Err(io::Error::from(io::ErrorKind::NotFound)),
        };
        let mut is_first = false;
        if let Some(current_monitor) = &self.reader {
            if current_monitor.file_path != latest_log_path {
                eprintln!(
                    "Changing reading log file: {}.",
                    latest_log_path.to_str().unwrap()
                );
                self.reader = Some(ContinuousFileReader::new(latest_log_path));
            }
        } else {
            // ログなしから新規作成されたものを読み出すとき
            eprintln!(
                "Starting reading log file: {}.",
                latest_log_path.to_str().unwrap()
            );
            self.reader = Some(ContinuousFileReader::new(latest_log_path));
            is_first = true;
        }
        if let Some(monitor) = &mut self.reader {
            return monitor.read_appended_lines(|line| {
                if let Some(log_line) = LogLine::from_line(line) {
                    self.processor.process_line(log_line, is_first);
                }
            });
        }
        Ok(())
    }
}

fn find_latest_log_path<I>(log_entries: I) -> Option<PathBuf>
where
    I: Iterator<Item = fs::DirEntry>,
{
    let entries = log_entries.filter_map(|entry| {
        let metadata = entry.metadata().ok()?;
        let modified_at = metadata.modified().ok()?;
        Some((entry, modified_at))
    });
    let latest_modified_entry =
        entries.max_by(|(_, modified_a), (_, modified_b)| modified_a.cmp(modified_b));
    if let Some((entry, _)) = latest_modified_entry {
        return Some(entry.path());
    }
    None
}
