use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use crate::error::{RustealError, RustealResult};
use crate::lock_or_recover;
use crate::sync::Mutex;

struct Slot<T> {
    value: Option<T>,
    closed: bool,
    waker: Option<Waker>,
}

/// The sending half of [`once`]: the first value sent is the one received.
pub struct OnceSender<T> {
    slot: Arc<Mutex<Slot<T>>>,
}

/// The receiving half of [`once`]: a future of the first value sent, or of an error when the
/// sender was dropped first (the delegate's object went away, say).
pub struct OnceReceiver<T> {
    slot: Arc<Mutex<Slot<T>>>,
}

/// A channel for one value, from a callback (a delegate, a timer) to the task awaiting it.
pub fn once<T>() -> (OnceSender<T>, OnceReceiver<T>) {
    let slot = Arc::new(Mutex::new(Slot {
        value: None,
        closed: false,
        waker: None,
    }));

    (OnceSender { slot: slot.clone() }, OnceReceiver { slot })
}

impl<T> OnceSender<T> {
    pub fn send(&self, value: T) {
        let waker = {
            let mut slot = lock_or_recover(&self.slot);

            if slot.value.is_some() || slot.closed {
                return;
            }

            slot.value = Some(value);
            slot.waker.take()
        };

        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl<T> Drop for OnceSender<T> {
    fn drop(&mut self) {
        let waker = {
            let mut slot = lock_or_recover(&self.slot);
            slot.closed = true;
            slot.waker.take()
        };

        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl<T> Future for OnceReceiver<T> {
    type Output = RustealResult<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<RustealResult<T>> {
        let mut slot = lock_or_recover(&self.slot);

        if let Some(value) = slot.value.take() {
            return Poll::Ready(Ok(value));
        }

        if slot.closed {
            return Poll::Ready(Err(
                RustealError::object_destroyed().context("the delegate went away before it fired")
            ));
        }

        slot.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::task::Wake;

    struct Noop;

    impl Wake for Noop {
        fn wake(self: Arc<Self>) {}
    }

    fn poll<T>(receiver: &mut OnceReceiver<T>) -> Poll<RustealResult<T>> {
        let waker = Waker::from(Arc::new(Noop));
        Pin::new(receiver).poll(&mut Context::from_waker(&waker))
    }

    #[test]
    fn the_first_value_is_received_and_a_dropped_sender_is_an_error() {
        let (sender, mut receiver) = once();
        assert!(poll(&mut receiver).is_pending());

        sender.send(1);
        sender.send(2);
        assert!(matches!(poll(&mut receiver), Poll::Ready(Ok(1))));

        let (sender, mut receiver) = once::<i32>();
        drop(sender);
        assert!(matches!(poll(&mut receiver), Poll::Ready(Err(_))));
    }
}
