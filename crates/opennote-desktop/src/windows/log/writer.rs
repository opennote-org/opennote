use std::{
    io::{self, Write},
    sync::mpsc::SyncSender,
};

use tracing_subscriber::fmt::MakeWriter;

/// Feed the standard tracing formatter's output into the window channel.
pub struct WindowLogWriter {
    pub sender: SyncSender<String>,
}

impl WindowLogWriter {
    pub fn new(sender: SyncSender<String>) -> Self {
        Self { sender }
    }
}

pub struct LogEventWriter {
    sender: SyncSender<String>,
    bytes: Vec<u8>,
}

impl<'a> MakeWriter<'a> for WindowLogWriter {
    type Writer = LogEventWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LogEventWriter {
            sender: self.sender.clone(),
            bytes: Vec::new(),
        }
    }
}

impl Write for LogEventWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for LogEventWriter {
    fn drop(&mut self) {
        // Send the whole event, preserving embedded newlines, without blocking.
        if !self.bytes.is_empty() {
            let _ = self
                .sender
                .try_send(String::from_utf8_lossy(&self.bytes).into_owned());
        }
    }
}
