mod shardeddata;
use std::{pin::Pin, task::{Context, Poll}, time::{Duration, Instant}};

pub use shardeddata::ShardedLockedData;

pub struct AsyncSleepFuture {
    start: Instant,
    dur: Duration,
}

impl std::future::Future for AsyncSleepFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        if self.start.elapsed() >= self.dur {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub async fn sleep(dur:Duration) -> AsyncSleepFuture {
    AsyncSleepFuture {
        dur,
        start: std::time::Instant::now(),
    }
}
