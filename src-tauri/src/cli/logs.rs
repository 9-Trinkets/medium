use anyhow::Result;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::thread;
use std::time::Duration;

pub fn run(lines_count: usize, filter: Option<&str>, follow: bool) -> Result<()> {
    let log_path = tauri_app_lib::config::log_file_path()?;
    if !log_path.exists() {
        println!("No logs found at {:?}", log_path);
        return Ok(());
    }

    print_recent_lines(&log_path, lines_count, filter)?;

    if follow {
        follow_logs(&log_path, filter)?;
    }

    Ok(())
}

fn print_recent_lines(path: &Path, lines_count: usize, filter: Option<&str>) -> Result<()> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut tail = VecDeque::with_capacity(lines_count.max(1));

    for line in reader.lines() {
        let line = line?;
        if matches_filter(&line, filter) {
            if tail.len() == lines_count.max(1) {
                tail.pop_front();
            }
            tail.push_back(line);
        }
    }

    for line in tail {
        println!("{}", line);
    }

    Ok(())
}

fn follow_logs(path: &Path, filter: Option<&str>) -> Result<()> {
    let mut file = File::open(path)?;
    let mut position = file.metadata()?.len();

    loop {
        let metadata = match std::fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(_) => {
                thread::sleep(Duration::from_millis(400));
                continue;
            }
        };

        if metadata.len() < position {
            position = 0;
        }

        if metadata.len() > position {
            file.seek(SeekFrom::Start(position))?;
            let mut reader = BufReader::new(&file);
            let mut buffer = String::new();
            while reader.read_line(&mut buffer)? > 0 {
                let line = buffer.trim_end_matches(['\n', '\r']);
                if matches_filter(line, filter) {
                    println!("{}", line);
                }
                buffer.clear();
            }
            position = file.stream_position()?;
        }

        thread::sleep(Duration::from_millis(400));
    }
}

fn matches_filter(line: &str, filter: Option<&str>) -> bool {
    filter
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none_or(|needle| line.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::matches_filter;

    #[test]
    fn matches_all_when_filter_missing() {
        assert!(matches_filter("hello world", None));
        assert!(matches_filter("hello world", Some("")));
    }

    #[test]
    fn matches_substring_filter() {
        assert!(matches_filter("[INFO] started", Some("INFO")));
        assert!(!matches_filter("[ERROR] failed", Some("INFO")));
    }
}
