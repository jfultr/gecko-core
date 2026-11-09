//! A timestamped event log kept in memory for the screen and appended to a file
//! as evidence.

use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Lines kept in memory for the screen; the file keeps everything.
const KEPT: usize = 2000;

/// Current UTC time of day as `HH:MM:SS.mmm`.
#[must_use]
pub fn clock() -> String {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let millis = since.as_millis() % 86_400_000;
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        millis / 3_600_000,
        millis / 60_000 % 60,
        millis / 1000 % 60,
        millis % 1000
    )
}

/// Seconds since the Unix epoch, for unique file names.
#[must_use]
pub fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// A shared log: each line gets a UTC timestamp, stays in memory (the last
/// lines) and is appended to the file, if one is open.
#[derive(Debug)]
pub struct LogBook {
    lines: Mutex<VecDeque<String>>,
    file: Mutex<Option<File>>,
    path: Option<PathBuf>,
}

impl LogBook {
    /// A log kept only in memory.
    #[must_use]
    pub fn in_memory() -> Self {
        Self {
            lines: Mutex::default(),
            file: Mutex::new(None),
            path: None,
        }
    }

    /// A log that also appends to `path`, creating its directory.
    ///
    /// # Errors
    ///
    /// The I/O error creating the directory or opening the file.
    pub fn to_file(path: &Path) -> io::Result<Self> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            lines: Mutex::default(),
            file: Mutex::new(Some(file)),
            path: Some(path.to_path_buf()),
        })
    }

    /// The file this log appends to, if any.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Adds one line with the current time.
    pub fn push(&self, line: impl AsRef<str>) {
        let line = format!("{} {}", clock(), line.as_ref());
        if let Ok(mut file) = self.file.lock()
            && let Some(file) = file.as_mut()
        {
            let _ = writeln!(file, "{line}");
        }
        if let Ok(mut lines) = self.lines.lock() {
            if lines.len() == KEPT {
                lines.pop_front();
            }
            lines.push_back(line);
        }
    }

    /// The last `count` lines, oldest first.
    #[must_use]
    pub fn tail(&self, count: usize) -> Vec<String> {
        self.lines.lock().map_or_else(
            |_| Vec::new(),
            |lines| {
                let skip = lines.len().saturating_sub(count);
                lines.iter().skip(skip).cloned().collect()
            },
        )
    }

    /// Every line kept in memory, oldest first.
    #[must_use]
    pub fn all(&self) -> Vec<String> {
        self.tail(KEPT)
    }
}
