mod output;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal,
};
use std::time::Duration;

use self::output::Output;

const QUIT_TIMES: u8 = 3;

pub struct Editor {
    reader: Reader,
    output: Output,
    quit_times: u8,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            reader: Reader,
            output: Output::new(),
            quit_times: QUIT_TIMES,
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
                ..
            } => {
                if self.output.is_dirty() && self.quit_times > 0 {
                    self.output.set_message(format!(
                        "WARNING!!! File has unsaved changes. Press Ctrl-Q {} more times to quit.",
                        self.quit_times
                    ));
                    self.quit_times -= 1;
                    return Ok(true);
                }
                return Ok(false);
            }
            KeyEvent {
                code: KeyCode::Char('s'),
                modifiers: event::KeyModifiers::CONTROL,
                ..
            } => self.output.save()?,
            KeyEvent {
                code:
                    direction @ (KeyCode::Up
                    | KeyCode::Left
                    | KeyCode::Down
                    | KeyCode::Right
                    | KeyCode::Home
                    | KeyCode::End),
                modifiers: KeyModifiers::NONE,
                ..
            } => self.output.move_cursor(direction),
            KeyEvent {
                code: val @ (KeyCode::PageUp | KeyCode::PageDown),
                modifiers: KeyModifiers::NONE,
                ..
            } => self.output.page_up_down(val),
            KeyEvent {
                code: code @ (KeyCode::Char(..) | KeyCode::Tab),
                modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                ..
            } => self.output.insert_char(match code {
                KeyCode::Tab => '\t',
                KeyCode::Char(ch) => ch,
                _ => unimplemented!(),
            }),
            KeyEvent {
                code: key @ (KeyCode::Backspace | KeyCode::Delete),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                if matches!(key, KeyCode::Delete) {
                    self.output.move_cursor(KeyCode::Right)
                }
                self.output.delete_char();
            }
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.output.insert_newline(),
            _ => {}
        }
        self.quit_times = QUIT_TIMES;
        Ok(true)
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not turn off raw mode");
        Output::clear_screen().expect("Error");
    }
}

pub struct Reader;

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

#[macro_export]
macro_rules! prompt {
    ($output:expr,$($args:tt)*) => {{
        let output:&mut Output = $output;
        let mut input = String::with_capacity(32);
        loop {
            output.set_message(format!($($args)*, input));
            output.refresh_screen()?;
            match Reader.read_key()? {
                KeyEvent {
                    code:KeyCode::Enter,
                    modifiers:KeyModifiers::NONE,
                    ..
                } => {
                    if !input.is_empty() {
                        output.set_message(String::new());
                        break;
                    }
                }
                KeyEvent {
                    code: KeyCode::Esc,
                    ..
                } => {
                    output.set_message(String::new());
                    input.clear();
                    break;
                }
                KeyEvent {
                    code: KeyCode::Backspace | KeyCode::Delete,
                    modifiers: KeyModifiers::NONE,
                    ..
                } =>  {
                    input.pop();
                }
                KeyEvent {
                    code: code @ (KeyCode::Char(..) | KeyCode::Tab),
                    modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                    ..
                } => input.push(match code {
                        KeyCode::Tab => '\t',
                        KeyCode::Char(ch) => ch,
                        _ => unreachable!(),
                    }),
                _=> {}
            }
        }
        if input.is_empty() { None } else { Some (input) }
    }};
}
