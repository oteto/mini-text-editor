mod output;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal,
};
use std::time::Duration;

use self::output::Output;

pub struct Editor {
    reader: Reader,
    output: Output,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            reader: Reader,
            output: Output::new(),
        }
    }

    pub fn init(&self) -> crossterm::Result<()> {
        terminal::enable_raw_mode()?;
        Ok(())
    }

    pub fn run(&mut self) -> crossterm::Result<bool> {
        self.output.refresh_screen()?;
        self.process_keypress()
    }

    fn process_keypress(&mut self) -> crossterm::Result<bool> {
        match self.reader.read_key()? {
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: event::KeyModifiers::CONTROL,
                kind: _,
                state: _,
            } => return Ok(false),
            KeyEvent {
                code:
                    direction @ (KeyCode::Up
                    | KeyCode::Left
                    | KeyCode::Down
                    | KeyCode::Right
                    | KeyCode::Home
                    | KeyCode::End),
                modifiers: KeyModifiers::NONE,
                kind: _,
                state: _,
            } => self.output.move_cursor(direction),
            KeyEvent {
                code: val @ (KeyCode::PageUp | KeyCode::PageDown),
                modifiers: KeyModifiers::NONE,
                kind: _,
                state: _,
            } => self.output.page_up_down(val),
            _ => {}
        }
        Ok(true)
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not turn off raw mode");
        Output::clear_screen().expect("Error");
    }
}

struct Reader;

impl Reader {
    pub fn read_key(&self) -> crossterm::Result<KeyEvent> {
        loop {
            if !event::poll(Duration::from_millis(500))? {
                continue;
            }

            if let Event::Key(event) = event::read()? {
                return Ok(event);
            }
        }
    }
}
