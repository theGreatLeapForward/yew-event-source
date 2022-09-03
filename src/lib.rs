//! Event stream handling for the yew framework
use std::fmt;

use gloo_events::EventListener;
use std::borrow::Cow;
use wasm_bindgen::JsCast;
use web_sys::{Event, EventSource, MessageEvent};
use yew::callback::Callback;

/// A status of an event source connection. Used for status notification.
#[derive(PartialEq, Debug)]
pub enum EventSourceStatus {
    /// Fired when an event source connection was opened.
    Open,
    /// Fired when an event source connection had an error.
    Error,
}

/// Ready state of an event source
///
/// [Documented at MDN](https://developer.mozilla.org/en-US/docs/Web/API/EventSource/readyState)
#[derive(PartialEq, Debug)]
pub enum ReadyState {
    /// The event source connection is connecting.
    Connecting,
    /// The event source connection is open.
    Open,
    /// The event source connection is closed.
    Closed,
}

/// A handle to control current event source connection.
pub struct EventSourceTask {
    event_source: EventSource,
    // We need to keep this else it is cleaned up on drop.
    _notification: Callback<EventSourceStatus>,
    listeners: Vec<EventListener>,
}

impl EventSourceTask {
    #![allow(clippy::unnecessary_wraps)]
    fn new(
        event_source: EventSource,
        notification: Callback<EventSourceStatus>,
    ) -> Result<EventSourceTask, &'static str> {
        Ok(EventSourceTask {
            event_source,
            _notification: notification,
            listeners: vec![],
        })
    }

    fn add_unwrapped_event_listener<S, F>(&mut self, event_type: S, callback: F)
    where
        S: Into<Cow<'static, str>>,
        F: FnMut(&Event) + 'static,
    {
        self.listeners
            .push(EventListener::new(&self.event_source, event_type, callback));
    }

    /// Register a callback for events of a given type
    ///
    /// This registers an event listener, which will fire `callback` when an
    /// event of `event_type` occurs.
    pub fn add_event_listener<S, OUT: 'static>(&mut self, event_type: S, callback: Callback<OUT>)
    where
        S: Into<Cow<'static, str>>,
        OUT: From<Result<String, String>>,
    {
        // This will convert from a generic `Event` into a `MessageEvent` taking
        // text, as is required by an event source.
        let wrapped_callback = move |event: &Event| {
            let event = event.dyn_ref::<MessageEvent>().unwrap();
            let text = event.data().as_string();

            let data = if let Some(text) = text {
                Ok(text)
            } else {
                Err("sus".to_string())
            };

            let out = OUT::from(data);
            callback.emit(out);
        };
        self.add_unwrapped_event_listener(event_type, wrapped_callback);
    }

    /// Query the ready state of the event source.
    pub fn ready_state(&self) -> ReadyState {
        match self.event_source.ready_state() {
            web_sys::EventSource::CONNECTING => ReadyState::Connecting,
            web_sys::EventSource::OPEN => ReadyState::Open,
            web_sys::EventSource::CLOSED => ReadyState::Closed,
            _ => panic!("unexpected ready state"),
        }
    }
}

impl fmt::Debug for EventSourceTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("EventSourceTask")
    }
}

/// An event source service attached to a user context.
#[derive(Default, Debug)]
pub struct EventSourceService {}

impl EventSourceService {
    /// Creates a new service instance.
    pub fn new() -> Self {
        Self {}
    }

    /// Connects to a server at `url` by an event source connection.
    ///
    /// The `notification` callback is fired when either an open or error event
    /// happens.
    pub fn connect(
        &mut self,
        url: &str,
        notification: Callback<EventSourceStatus>,
    ) -> Result<EventSourceTask, &str> {
        let event_source = EventSource::new(url);
        if event_source.is_err() {
            return Err("Failed to created event source with given URL");
        }

        let event_source = event_source.map_err(|_| "failed to build event source")?;

        let notify = notification.clone();
        let listener_open = move |_: &Event| {
            notify.emit(EventSourceStatus::Open);
        };
        let notify = notification.clone();
        let listener_error = move |_: &Event| {
            notify.emit(EventSourceStatus::Error);
        };

        let mut result = EventSourceTask::new(event_source, notification)?;
        result.add_unwrapped_event_listener("open", listener_open);
        result.add_unwrapped_event_listener("error", listener_error);
        Ok(result)
    }
}
