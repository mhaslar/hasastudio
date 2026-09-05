use crossbeam_queue::ArrayQueue;
use rezie_core::{FrameTime, OutputId, SinkStats};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

struct Queue {
    id: OutputId,
    frames: ArrayQueue<FrameTime>,
    dispatched: AtomicU64,
    dropped: AtomicU64,
}

/// Single producer retained by the clock thread.
pub struct TickProducer(Arc<Queue>);
/// Single consumer retained by one sink or the acceptance harness.
pub struct TickConsumer(Arc<Queue>);

/// Preallocate one bounded sink; zero capacity is rejected.
pub fn tick_sink(
    id: OutputId,
    capacity: usize,
) -> Result<(TickProducer, TickConsumer), crate::EngineError> {
    if capacity == 0 {
        return Err(crate::EngineError::Configuration(
            "sink capacity must be positive".into(),
        ));
    }
    let queue = Arc::new(Queue {
        id,
        frames: ArrayQueue::new(capacity),
        dispatched: AtomicU64::new(0),
        dropped: AtomicU64::new(0),
    });
    Ok((TickProducer(queue.clone()), TickConsumer(queue)))
}

impl TickProducer {
    /// Atomically overwrite the oldest tick on overflow; never wait for a consumer.
    pub fn dispatch(&mut self, frame: FrameTime) {
        if self.0.frames.force_push(frame).is_some() {
            self.0.dropped.fetch_add(1, Ordering::Relaxed);
        }
        self.0.dispatched.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn observer(&self) -> TickConsumer {
        TickConsumer(self.0.clone())
    }
}

impl TickConsumer {
    /// Consume the oldest available tick without waiting.
    pub fn pop(&mut self) -> Option<FrameTime> {
        self.0.frames.pop()
    }

    /// Read approximate live counters, exact after the clock stops.
    pub fn stats(&self) -> SinkStats {
        SinkStats {
            id: self.0.id,
            dispatched: self.0.dispatched.load(Ordering::Relaxed),
            dropped: self.0.dropped.load(Ordering::Relaxed),
            queued: self.0.frames.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn slow_sink_drops_its_own_oldest_and_keeps_latest_ticks() {
        let (mut slow, mut a) = tick_sink(OutputId(1), 2).unwrap();
        let (mut fast, mut b) = tick_sink(OutputId(2), 2).unwrap();
        for index in 0..100 {
            let tick = FrameTime {
                index,
                pts: Duration::from_millis(index * 20),
            };
            slow.dispatch(tick);
            fast.dispatch(tick);
            assert_eq!(b.pop(), Some(tick));
        }
        assert_eq!(a.stats().dropped, 98);
        assert_eq!(b.stats().dropped, 0);
        assert_eq!(a.pop().unwrap().index, 98);
        assert_eq!(a.pop().unwrap().index, 99);
        assert!(a.pop().is_none());
    }
}
