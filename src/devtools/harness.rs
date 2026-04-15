use crossterm::event::Event;
use std::collections::VecDeque;
use std::io;
use std::time::Duration;

pub trait EventSource: Send {
    fn poll(&mut self, timeout: Duration) -> io::Result<bool>;
    fn read(&mut self) -> io::Result<Event>;
}

#[derive(Debug, Default)]
pub struct CrosstermEventSource;

impl EventSource for CrosstermEventSource {
    fn poll(&mut self, timeout: Duration) -> io::Result<bool> {
        crossterm::event::poll(timeout)
    }

    fn read(&mut self) -> io::Result<Event> {
        crossterm::event::read()
    }
}

#[derive(Debug, Default)]
pub struct ScriptedEventSource {
    events: VecDeque<Event>,
}

impl ScriptedEventSource {
    pub fn new(events: impl IntoIterator<Item = Event>) -> Self {
        Self {
            events: events.into_iter().collect(),
        }
    }

    pub fn push(&mut self, event: Event) {
        self.events.push_back(event);
    }
}

impl EventSource for ScriptedEventSource {
    fn poll(&mut self, _timeout: Duration) -> io::Result<bool> {
        Ok(!self.events.is_empty())
    }

    fn read(&mut self) -> io::Result<Event> {
        self.events.pop_front().ok_or_else(|| {
            io::Error::new(io::ErrorKind::WouldBlock, "scripted event source is empty")
        })
    }
}
