use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use futures::{Stream, StreamExt};
use futures_ticker::Ticker;

pub struct Heartbeat {
    /// Heartbeat interval stream.
    ticker: Ticker,

    /// Number of heartbeats since the beginning of time.
    ticks: u64,
}

impl Heartbeat {
    pub fn new(interval: Duration, delay: Duration) -> Self {
        Self {
            ticker: Ticker::new_with_next(interval, delay),
            ticks: 0,
        }
    }
}

impl Stream for Heartbeat {
    type Item = u64;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.ticker.poll_next_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(_)) => {
                self.ticks = self.ticks.wrapping_add(1);
                Poll::Ready(Some(self.ticks))
            }
            Poll::Ready(None) => Poll::Ready(None),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ticker.size_hint()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_heartbeat_ticks() {
        //// Given
        let mut hb = Heartbeat::new(Duration::from_millis(100), Duration::from_millis(0));

        //// When
        let tick1 = hb.next().await;
        let tick2 = hb.next().await;
        let tick3 = hb.next().await;

        //// Then
        assert_eq!(tick1, Some(1));
        assert_eq!(tick2, Some(2));
        assert_eq!(tick3, Some(3));
    }

    #[tokio::test]
    async fn test_heartbeat_ticks_wrapping() {
        //// Given
        let mut hb = Heartbeat::new(Duration::from_millis(100), Duration::from_millis(0));
        hb.ticks = u64::MAX - 1;

        //// When
        let tick1 = hb.next().await;
        let tick2 = hb.next().await;
        let tick3 = hb.next().await;

        //// Then
        assert_eq!(tick1, Some(u64::MAX));
        assert_eq!(tick2, Some(0));
        assert_eq!(tick3, Some(1));
    }
}
