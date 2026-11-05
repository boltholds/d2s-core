#![doc = "D2S Phase 0: a Rust API over Fast DDS through a narrow CXX adapter."]
#![forbid(unsafe_code)]

use std::error::Error as StdError;
use std::fmt;
use std::time::Duration;
use tokio::time::{Instant, sleep};

pub const PHASE0_TOPIC: &str = "d2s.phase0.hello";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub index: u32,
    pub text: String,
}

#[derive(Debug)]
pub struct Error {
    description: String,
}

impl Error {
    fn from_native(description: String) -> Self {
        Self { description }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.description)
    }
}

impl StdError for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// A single DDS writer with its own participant. Not thread-shareable in Phase 0.
pub struct Publisher {
    native: d2s_sys::Publisher,
}

impl Publisher {
    pub fn new(domain_id: u32, topic: &str) -> Result<Self> {
        d2s_sys::Publisher::open(domain_id, topic)
            .map(|native| Self { native })
            .map_err(Error::from_native)
    }

    /// Returns after Fast DDS write, not after application processing.
    pub fn publish(&mut self, message: &Message) -> Result<()> {
        self.native
            .publish(message.index, &message.text)
            .map_err(Error::from_native)
    }

    /// Wait for DDS discovery to match at least one reader.
    pub async fn wait_for_readers(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            if self.native.has_readers() {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            sleep(Duration::from_millis(20)).await;
        }
    }
}

/// A single DDS reader with its own participant.
pub struct Subscriber {
    native: d2s_sys::Subscriber,
}

impl Subscriber {
    pub fn new(domain_id: u32, topic: &str) -> Result<Self> {
        d2s_sys::Subscriber::open(domain_id, topic)
            .map(|native| Self { native })
            .map_err(Error::from_native)
    }

    /// Poll one DDS sample. None means that the reader has no pending valid sample.
    pub fn try_receive(&mut self) -> Result<Option<Message>> {
        self.native
            .try_receive()
            .map(|sample| {
                sample.map(|received| Message {
                    index: received.index,
                    text: received.text,
                })
            })
            .map_err(Error::from_native)
    }

    /// Async convenience wrapper. Phase 0 uses bounded-interval polling.
    pub async fn receive_timeout(&mut self, timeout: Duration) -> Result<Option<Message>> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(message) = self.try_receive()? {
                return Ok(Some(message));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            sleep(Duration::from_millis(20)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_keeps_utf8_data() {
        let message = Message {
            index: 1,
            text: String::from("Привет, DDS"),
        };
        assert_eq!(message.index, 1);
        assert_eq!(message.text, "Привет, DDS");
    }
}
