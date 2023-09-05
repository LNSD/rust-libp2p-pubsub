use bytes::Bytes;

#[derive(Debug)]
pub enum Command {
    /// A pubsub frame to send to the remote.
    SendFrame(Bytes),
}

#[derive(Debug)]
pub enum Event {
    /// A pubsub frame has been received.
    FrameReceived(Bytes),
}

/// Events consumed by substream handlers.
#[derive(Debug)]
pub enum StreamHandlerIn<TStream> {
    /// Initializes a substream handler with the provided stream.
    ///
    /// This event is emitted when a new substream is fully negotiated.
    Init(TStream),

    /// A pubsub frame to send to the remote.
    SendFrame(Bytes),
}

/// Events emitted by substream handlers.
#[derive(Debug)]
pub enum StreamHandlerOut {
    /// A pubsub frame has been received.
    FrameReceived(Bytes),
}
