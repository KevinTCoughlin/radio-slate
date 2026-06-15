use std::io::{BufWriter, Write};
use std::path::Path;

/// Abstraction over where emitted output lines are written.
///
/// Implementations include [`StdoutSink`] for interactive/pipe use, [`FileSink`]
/// for disk serialization, and [`BufferedSink`] for in-process testing.
pub trait OutputSink {
    fn emit(&mut self, line: &str) -> anyhow::Result<()>;
    fn flush(&mut self) -> anyhow::Result<()>;
}

/// Buffered writer that emits lines to stdout.
#[derive(Debug)]
pub struct StdoutSink {
    inner: BufWriter<std::io::Stdout>,
}

impl Default for StdoutSink {
    fn default() -> Self {
        Self {
            inner: BufWriter::new(std::io::stdout()),
        }
    }
}

impl StdoutSink {
    pub fn new() -> Self {
        Self::default()
    }
}

impl OutputSink for StdoutSink {
    fn emit(&mut self, line: &str) -> anyhow::Result<()> {
        writeln!(self.inner, "{line}")?;
        Ok(())
    }

    fn flush(&mut self) -> anyhow::Result<()> {
        self.inner.flush()?;
        Ok(())
    }
}

/// Buffered writer that emits lines to a file on disk.
#[derive(Debug)]
pub struct FileSink {
    inner: BufWriter<std::fs::File>,
}

impl FileSink {
    /// Create (or truncate) the file at `path` and wrap it in a buffered sink.
    pub fn create(path: &Path) -> anyhow::Result<Self> {
        let file = std::fs::File::create(path).map_err(|e| {
            anyhow::anyhow!("failed to create output file '{}': {e}", path.display())
        })?;
        Ok(Self {
            inner: BufWriter::new(file),
        })
    }
}

impl OutputSink for FileSink {
    fn emit(&mut self, line: &str) -> anyhow::Result<()> {
        writeln!(self.inner, "{line}")?;
        Ok(())
    }

    fn flush(&mut self) -> anyhow::Result<()> {
        self.inner.flush()?;
        Ok(())
    }
}

/// In-memory sink that accumulates emitted lines.  Intended for testing.
#[derive(Default)]
pub struct BufferedSink {
    pub lines: Vec<String>,
}

impl BufferedSink {
    pub fn new() -> Self {
        Self::default()
    }
}

impl OutputSink for BufferedSink {
    fn emit(&mut self, line: &str) -> anyhow::Result<()> {
        self.lines.push(line.to_string());
        Ok(())
    }

    fn flush(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffered_sink_captures_emitted_lines() {
        let mut sink = BufferedSink::new();
        sink.emit("first").unwrap();
        sink.emit("second").unwrap();
        assert_eq!(sink.lines, vec!["first", "second"]);
    }

    #[test]
    fn buffered_sink_flush_is_infallible() {
        let mut sink = BufferedSink::new();
        assert!(sink.flush().is_ok());
    }

    #[test]
    fn buffered_sink_starts_empty() {
        let sink = BufferedSink::new();
        assert!(sink.lines.is_empty());
    }

    #[test]
    fn file_sink_writes_and_flushes_to_disk() {
        let path = std::env::temp_dir().join("radio_slate_output_test.json");
        let mut sink = FileSink::create(&path).unwrap();
        sink.emit(r#"{"state":"stopped"}"#).unwrap();
        sink.flush().unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.trim(), r#"{"state":"stopped"}"#);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn file_sink_fails_on_invalid_path() {
        let bad_path = Path::new("/nonexistent_dir/radio_slate_test.json");
        let result = FileSink::create(bad_path);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("failed to create output file"));
    }
}
