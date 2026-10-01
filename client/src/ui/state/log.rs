//! The server log: connections, joins, voice state and errors, one line each.

/// What a log line reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// Something the client did.
    Info,
    /// Something that happened on the server.
    Event,
    /// Something that failed.
    Error,
}

impl Tone {
    /// The status tone a line's mark is painted with.
    pub fn token(self) -> &'static str {
        match self {
            Tone::Info => "info",
            Tone::Event => "idle",
            Tone::Error => "err",
        }
    }
}

/// One line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogLine {
    pub id: u64,
    pub time: String,
    pub tone: Tone,
    pub text: String,
}

/// How many lines are kept.
const KEEP: usize = 1000;

/// The lines, oldest first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Log {
    pub lines: Vec<LogLine>,
    next: u64,
}

impl Log {
    pub fn push(&mut self, tone: Tone, text: String) {
        self.next += 1;
        self.lines.push(LogLine {
            id: self.next,
            time: chrono::Local::now().format("%H:%M:%S").to_string(),
            tone,
            text,
        });
        if self.lines.len() > KEEP {
            self.lines.drain(..KEEP / 4);
        }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }
}
