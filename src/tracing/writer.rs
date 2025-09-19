//! Writer implementation for collecting tracing events and sending them to the collector.

use std::io;
use futures::channel::mpsc;
use tracing_subscriber::fmt::MakeWriter;

use crate::runner::basic::ScenarioId;
use super::{
    types::{LogMessage, LogSender},
    formatter::suffix,
};

/// [`io::Write`]r sending [`tracing::Event`]s to a `Collector`.
#[derive(Clone, Debug)]
pub struct CollectorWriter {
    /// Sender for notifying the [`Collector`] about [`tracing::Event`]s.
    ///
    /// [`Collector`]: super::collector::Collector
    sender: LogSender,
}

impl CollectorWriter {
    /// Creates a new [`CollectorWriter`].
    pub const fn new(sender: LogSender) -> Self {
        Self { sender }
    }
}

impl<'a> MakeWriter<'a> for CollectorWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl io::Write for CollectorWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Although this is not documented explicitly anywhere, `io::Write`rs
        // inside `tracing::fmt::Layer` always receives fully formatted messages
        // at once, not by parts.
        // Inside docs of `fmt::Layer::with_writer()`, a non-locked `io::stderr`
        // is passed as an `io::Writer`. So, if this guarantee fails, parts of
        // log messages will be able to interleave each other, making the result
        // unreadable.
        let msgs = String::from_utf8_lossy(buf);
        for msg in msgs.split_terminator(suffix::END) {
            if let Some((before, after)) =
                msg.rsplit_once(suffix::NO_SCENARIO_ID)
            {
                if !after.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "wrong separator",
                    ));
                }
                _ = self.sender.unbounded_send((None, before.to_owned())).ok();
            } else if let Some((before, after)) =
                msg.rsplit_once(suffix::BEFORE_SCENARIO_ID)
            {
                let scenario_id = after.parse().map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, e)
                })?;
                _ = self
                    .sender
                    .unbounded_send((Some(scenario_id), before.to_owned()))
                    .ok();
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "missing separator",
                ));
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::TryStreamExt;
    use std::io::Write;

    #[test]
    fn test_collector_writer_creation() {
        let (sender, _receiver) = mpsc::unbounded();
        let writer = CollectorWriter::new(sender);
        
        // Test that the writer was created successfully
        assert!(std::mem::size_of_val(&writer) > 0);
    }

    #[test]
    fn test_make_writer() {
        let (sender, _receiver) = mpsc::unbounded();
        let writer = CollectorWriter::new(sender);
        
        let made_writer = writer.make_writer();
        assert!(std::mem::size_of_val(&made_writer) > 0);
    }

    #[test]
    fn test_write_message_without_scenario_id() -> io::Result<()> {
        let (sender, mut receiver) = mpsc::unbounded();
        let mut writer = CollectorWriter::new(sender);
        
        let message = format!("test log message{}{}", suffix::NO_SCENARIO_ID, suffix::END);
        let written = writer.write(message.as_bytes())?;
        
        assert_eq!(written, message.len());
        
        // Check that the message was sent correctly
        let (scenario_id, content) = receiver.try_next().unwrap().unwrap();
        assert!(scenario_id.is_none());
        assert_eq!(content, "test log message");
        
        Ok(())
    }

    #[test]
    fn test_write_message_with_scenario_id() -> io::Result<()> {
        let (sender, mut receiver) = mpsc::unbounded();
        let mut writer = CollectorWriter::new(sender);
        
        let message = format!(
            "test log message{}{}{}",
            suffix::BEFORE_SCENARIO_ID,
            "42",
            suffix::END
        );
        let written = writer.write(message.as_bytes())?;
        
        assert_eq!(written, message.len());
        
        // Check that the message was sent correctly
        let (scenario_id, content) = receiver.try_next().unwrap().unwrap();
        assert_eq!(scenario_id, Some(ScenarioId(42)));
        assert_eq!(content, "test log message");
        
        Ok(())
    }

    #[test]
    fn test_write_multiple_messages() -> io::Result<()> {
        let (sender, mut receiver) = mpsc::unbounded();
        let mut writer = CollectorWriter::new(sender);
        
        let message1 = format!("message1{}{}", suffix::NO_SCENARIO_ID, suffix::END);
        let message2 = format!(
            "message2{}{}{}",
            suffix::BEFORE_SCENARIO_ID,
            "123",
            suffix::END
        );
        let combined = format!("{}{}", message1, message2);
        
        let written = writer.write(combined.as_bytes())?;
        assert_eq!(written, combined.len());
        
        // Check first message
        let (scenario_id1, content1) = receiver.try_next().unwrap().unwrap();
        assert!(scenario_id1.is_none());
        assert_eq!(content1, "message1");
        
        // Check second message
        let (scenario_id2, content2) = receiver.try_next().unwrap().unwrap();
        assert_eq!(scenario_id2, Some(ScenarioId(123)));
        assert_eq!(content2, "message2");
        
        Ok(())
    }

    #[test]
    fn test_write_invalid_separator_after_no_scenario() {
        let (sender, _receiver) = mpsc::unbounded();
        let mut writer = CollectorWriter::new(sender);
        
        let message = format!("test{}extra{}", suffix::NO_SCENARIO_ID, suffix::END);
        let result = writer.write(message.as_bytes());
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_write_invalid_scenario_id() {
        let (sender, _receiver) = mpsc::unbounded();
        let mut writer = CollectorWriter::new(sender);
        
        let message = format!(
            "test{}invalid_number{}",
            suffix::BEFORE_SCENARIO_ID,
            suffix::END
        );
        let result = writer.write(message.as_bytes());
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_write_missing_separator() {
        let (sender, _receiver) = mpsc::unbounded();
        let mut writer = CollectorWriter::new(sender);
        
        let message = format!("test message{}", suffix::END);
        let result = writer.write(message.as_bytes());
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_flush() -> io::Result<()> {
        let (sender, _receiver) = mpsc::unbounded();
        let mut writer = CollectorWriter::new(sender);
        
        writer.flush()?;
        Ok(())
    }

    #[test]
    fn test_write_with_closed_receiver() -> io::Result<()> {
        let (sender, receiver) = mpsc::unbounded();
        drop(receiver); // Close the receiver
        
        let mut writer = CollectorWriter::new(sender);
        
        let message = format!("test{}{}", suffix::NO_SCENARIO_ID, suffix::END);
        // This should still succeed even with closed receiver
        let written = writer.write(message.as_bytes())?;
        assert_eq!(written, message.len());
        
        Ok(())
    }

    #[test]
    fn test_write_empty_message() -> io::Result<()> {
        let (sender, _receiver) = mpsc::unbounded();
        let mut writer = CollectorWriter::new(sender);
        
        let written = writer.write(&[])?;
        assert_eq!(written, 0);
        
        Ok(())
    }

    #[test]
    fn test_write_utf8_handling() -> io::Result<()> {
        let (sender, mut receiver) = mpsc::unbounded();
        let mut writer = CollectorWriter::new(sender);
        
        let unicode_message = format!("тест 🎯 message{}{}", suffix::NO_SCENARIO_ID, suffix::END);
        let written = writer.write(unicode_message.as_bytes())?;
        
        assert_eq!(written, unicode_message.as_bytes().len());
        
        let (scenario_id, content) = receiver.try_next().unwrap().unwrap();
        assert!(scenario_id.is_none());
        assert_eq!(content, "тест 🎯 message");
        
        Ok(())
    }

    #[test]
    fn test_scenario_id_parsing_edge_cases() -> io::Result<()> {
        let (sender, mut receiver) = mpsc::unbounded();
        let mut writer = CollectorWriter::new(sender);
        
        // Test zero scenario ID
        let message = format!("test{}0{}", suffix::BEFORE_SCENARIO_ID, suffix::END);
        writer.write(message.as_bytes())?;
        
        let (scenario_id, content) = receiver.try_next().unwrap().unwrap();
        assert_eq!(scenario_id, Some(ScenarioId(0)));
        assert_eq!(content, "test");
        
        // Test large scenario ID
        let message = format!("test{}{}{}", suffix::BEFORE_SCENARIO_ID, u64::MAX, suffix::END);
        writer.write(message.as_bytes())?;
        
        let (scenario_id, content) = receiver.try_next().unwrap().unwrap();
        assert_eq!(scenario_id, Some(ScenarioId(u64::MAX)));
        assert_eq!(content, "test");
        
        Ok(())
    }
}