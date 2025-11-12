//! Pageshelf global event bus module
//!
//! This provides an event bus to help manage communication between components.

use std::sync::Arc;

use tokio::sync::broadcast;
use tracing::{Level, debug, instrument, span};

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    // Topic-Event naming
    /// Should be called when a new page at a location has been made available to the server.
    ///
    /// Can be called either on initial discovery, or when a new version becomes available (signaling a time to update things).
    PageDiscovered {
        owner: Arc<str>,
        project: Arc<str>,
        channel: Arc<str>,
    },
    /// Should be called when a page is no longer available to the server due to deletion.
    ///
    /// It should preferably not, however, be called when it's merely rendered *inaccessible* due to problems.
    PageRemoved {
        owner: Arc<str>,
        project: Arc<str>,
        channel: Arc<str>,
    },
}

/// Global event bus, based on Tokio tasks.
///
/// This helps server components respond to each other.
/// Requires a Tokio runtime to be set up to function properly.
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<Event>,
}

impl EventBus {
    /// Creates a new [EventBus] with a specific buffer size.
    ///
    /// A higher buffer allows more events to be buffered.
    pub fn new(buffer: usize) -> Self {
        let (sender, _) = broadcast::channel(buffer);
        Self { sender }
    }

    /// The callback is called for every published event.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use pageshelf_core::event::{EventBus, Event};
    /// # use tokio::runtime::Runtime;
    /// # let rt = Runtime::new().unwrap();
    /// # rt.block_on(async {
    ///
    /// let bus = EventBus::default();
    ///
    /// bus.subscribe(|event| {
    ///     println!("Hello, world!")
    /// });
    ///
    /// bus.publish(Event::PageDiscovered {
    ///    owner: Arc::from("alice"),
    ///    project: Arc::from("prometheus"),
    ///    channel: Arc::from("bob"),
    /// });
    /// # });
    /// ```
    #[instrument(skip(self, callback))]
    pub fn subscribe<F>(&self, callback: F)
    where
        F: Fn(Event) + Send + 'static,
    {
        let mut receiver = self.sender.subscribe();
        tokio::task::spawn(async move {
            while let Ok(event) = receiver.recv().await {
                debug!(?event, "Dispatching event to sync subscriber");
                callback(event.clone());
            }
        });
    }

    /// Subscribes to the event bus, calling back an asynchronous function when events are published.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use pageshelf_core::event::{EventBus, Event};
    /// # use tokio::runtime::Runtime;
    /// # let rt = Runtime::new().unwrap();
    /// # rt.block_on(async {
    ///
    /// let bus = EventBus::default();
    ///
    /// bus.subscribe_async(async move |event| {
    ///     println!("Hello, world!")
    /// });
    ///
    /// bus.publish(Event::PageDiscovered {
    ///    owner: Arc::from("alice"),
    ///    project: Arc::from("prometheus"),
    ///    channel: Arc::from("bob"),
    /// });
    /// # });
    /// ```
    #[instrument(skip(self, callback))]
    pub fn subscribe_async<F, Fut>(&self, callback: F)
    where
        F: Fn(Event) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let mut receiver = self.sender.subscribe();
        tokio::spawn(async move {
            while let Ok(event) = receiver.recv().await {
                debug!(?event, "Dispatching event to async subscriber");
                callback(event.clone()).await;
            }
        });
    }

    /// Publish an event asynchronously
    #[instrument(skip(self))]
    pub fn publish(&self, event: Event) {
        let span = span!(Level::DEBUG, "eventbus.publish", event = ?event);
        let _span_guard = span.enter();

        match self.sender.send(event) {
            Ok(_) => {
                debug!("Published event");
            }
            Err(e) => {
                debug!("Error publishing event: {e}");
            }
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(300)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::time::{Duration, sleep};

    fn sample_event(kind: &str) -> Event {
        match kind {
            "available" => Event::PageDiscovered {
                owner: Arc::from("alice"),
                project: Arc::from("book"),
                channel: Arc::from("main"),
            },
            _ => Event::PageRemoved {
                owner: Arc::from("bob"),
                project: Arc::from("notes"),
                channel: Arc::from("draft"),
            },
        }
    }

    #[tokio::test]
    async fn subscriber_receives_published_event() {
        let bus = EventBus::new(10);
        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received.clone();

        bus.subscribe(move |event| {
            received_clone.lock().unwrap().push(event);
        });

        let event = sample_event("available");
        bus.publish(event.clone());

        sleep(Duration::from_millis(50)).await;

        let list = received.lock().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0], event);
    }

    #[tokio::test]
    async fn multiple_subscribers_receive_same_event() {
        let bus = EventBus::new(10);
        let a = Arc::new(Mutex::new(Vec::new()));
        let b = Arc::new(Mutex::new(Vec::new()));

        let a_c = a.clone();
        let b_c = b.clone();

        bus.subscribe(move |e| a_c.lock().unwrap().push(e));
        bus.subscribe(move |e| b_c.lock().unwrap().push(e));

        let event = sample_event("available");
        bus.publish(event.clone());

        sleep(Duration::from_millis(50)).await;

        let a_l = a.lock().unwrap();
        let b_l = b.lock().unwrap();
        assert_eq!(a_l.len(), 1);
        assert_eq!(b_l.len(), 1);
        assert_eq!(a_l[0], b_l[0]);
        assert_eq!(a_l[0], event);
    }

    #[tokio::test]
    async fn handles_closed_channel_gracefully() {
        let bus = EventBus::new(1);
        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received.clone();

        bus.subscribe(move |e| received_clone.lock().unwrap().push(e));
        // Drop the sender to force closure
        drop(bus);

        sleep(Duration::from_millis(50)).await;
        // No panic expected; just ensure test runs cleanly
        assert!(received.lock().unwrap().len() <= 1);
    }

    #[tokio::test]
    async fn subscriber_can_lag_without_panicking() {
        let bus = EventBus::new(1); // small buffer to force lag
        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received.clone();

        bus.subscribe(move |e| {
            received_clone.lock().unwrap().push(e);
            // simulate slow processing
            std::thread::sleep(std::time::Duration::from_millis(30));
        });

        // Send multiple events quickly
        for _ in 0..5 {
            bus.publish(sample_event("available"));
        }

        sleep(Duration::from_millis(200)).await;

        let list = received.lock().unwrap();
        assert!(!list.is_empty(), "subscriber received at least one event");
    }
}
