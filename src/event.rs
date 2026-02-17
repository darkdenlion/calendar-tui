use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent};

pub fn poll_event(timeout: Duration) -> color_eyre::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

pub fn next_key_event(timeout: Duration) -> color_eyre::Result<Option<KeyEvent>> {
    loop {
        match poll_event(timeout)? {
            Some(Event::Key(key)) => return Ok(Some(key)),
            Some(_) => continue,
            None => return Ok(None),
        }
    }
}
