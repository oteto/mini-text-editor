use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute, terminal,
};
use std::{io::stdout, time::Duration};

pub struct Editor {
    reader: Reader,
}

impl Editor {
    pub fn new() -> Self {
        Self { reader: Reader }
    }

    pub fn init(&self) -> crossterm::Result<()> {
        terminal::enable_raw_mode()?;
        Ok(())
    }

    pub fn run(&self) -> crossterm::Result<bool> {
        self.process_keypress()
    }

    fn process_keypress(&self) -> crossterm::Result<bool> {
        match self.reader.read_key()? {
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: event::KeyModifiers::CONTROL,
                kind: _,
                state: _,
            } => return Ok(false),
            _ => {}
        }
        Ok(true)
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not turn off raw mode");
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

struct Output;

impl Output {
    fn new() -> Self {
        Self
    }

    fn clear_screen() -> crossterm::Result<()> {
        execute!(stdout(), terminal::Clear(terminal::ClearType::All))
    }

    fn refresh_screen(&self) -> crossterm::Result<()> {
        Self::clear_screen()
    }
}
