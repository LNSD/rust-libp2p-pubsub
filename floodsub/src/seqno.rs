use instant::SystemTime;

/// A trait for message sequence number generators.
pub trait MessageSeqNumberGenerator {
    fn next(&mut self) -> Option<u64>;
}

/// A strictly linearly increasing sequence number.
///
/// We start from the current time as unix timestamp in milliseconds.
#[derive(Debug)]
pub struct LinearSequenceNumber(u64);

impl LinearSequenceNumber {
    /// Creates a new linear sequence number generator.
    pub fn new() -> Self {
        let unix_timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("time to be linear")
            .as_nanos();

        Self(unix_timestamp as u64)
    }
}

impl MessageSeqNumberGenerator for LinearSequenceNumber {
    fn next(&mut self) -> Option<u64> {
        self.0 = self
            .0
            .checked_add(1)
            .expect("to not exhaust u64 space for sequence numbers");

        Some(self.0)
    }
}

/// A random sequence number generator.
#[derive(Debug)]
pub struct RandomSequenceNumber {
    _priv: (), // Hide the struct constructor.
}

impl RandomSequenceNumber {
    /// Creates a new random sequence number generator.
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

impl MessageSeqNumberGenerator for RandomSequenceNumber {
    fn next(&mut self) -> Option<u64> {
        Some(rand::random())
    }
}

/// A sequence number generator that always returns `None`.
#[derive(Debug)]
pub struct NoSequenceNumber {
    _priv: (), // Hide the struct constructor.
}

impl NoSequenceNumber {
    /// Creates a new sequence number generator that always returns `None`.
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

impl MessageSeqNumberGenerator for NoSequenceNumber {
    fn next(&mut self) -> Option<u64> {
        None
    }
}
