use std::fmt::{Debug, Formatter};

use bytes::Bytes;

pub enum Command {
    /// A pubsub frame to send to the remote.
    SendFrame(Bytes),
}

impl Debug for Command {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::SendFrame(_) => write!(f, "SendFrame(...)"),
        }
    }
}

pub enum Event {
    /// A pubsub frame has been received.
    FrameReceived(Bytes),

    /// The frame was sent.
    FrameSent,
}

impl Debug for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::FrameReceived(_) => write!(f, "FrameReceived(...)"),
            Event::FrameSent => write!(f, "FrameSent"),
        }
    }
}

/// Events consumed by substream handlers.
pub enum StreamHandlerIn {
    /// A pubsub frame to send to the remote.
    SendFrame(Bytes),
}

impl Debug for StreamHandlerIn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamHandlerIn::SendFrame(_) => write!(f, "SendFrame(...)"),
        }
    }
}

/// Events emitted by substream handlers.
pub enum StreamHandlerOut {
    /// A pubsub frame has been received.
    FrameReceived(Bytes),

    /// The fame was sent.
    FrameSent,
}

impl Debug for StreamHandlerOut {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamHandlerOut::FrameReceived(_) => write!(f, "FrameReceived(...)"),
            StreamHandlerOut::FrameSent => write!(f, "FrameSent"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StreamHandlerError {
    #[error("failed to send data on outbound stream: {0}")]
    SendDataFailed(std::io::Error),

    #[error("failed to flush stream: {0}")]
    FlushFailed(std::io::Error),

    #[error("failed to read data from stream: {0}")]
    ReadDataFailed(std::io::Error),

    #[error("failed to close stream: {0}")]
    CloseFailed(std::io::Error),

    #[error("stream closed by remote")]
    ClosedByRemote,
}
