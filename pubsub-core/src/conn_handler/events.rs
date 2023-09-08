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
