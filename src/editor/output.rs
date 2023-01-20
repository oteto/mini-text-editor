mod cursor;
mod row;
mod status;

use std::io::{self, stdout, Write};

use crossterm::{event::KeyCode, execute, queue, style, terminal};

use self::{cursor::CursorController, row::EditorRows, status::StatusMessage};

pub struct Output {
    win_size: (usize, usize),
    editor_contents: EditorContents,
    cursor_controller: CursorController,
    editor_rows: EditorRows,
    status_message: StatusMessage,
    dirty: u64,
}

impl Output {
    pub fn new() -> Self {
        let win_size = terminal::size()
            .map(|(x, y)| (x as usize, y as usize - 2))
            .unwrap();
        Self {
            win_size,
            editor_contents: EditorContents::new(),
            cursor_controller: CursorController::new(win_size),
            editor_rows: EditorRows::new(),
            status_message: StatusMessage::new("HELP: Ctrl-S = Save | Ctrl-Q = Quit ".into()),
            dirty: 0,
        }
    }

    pub fn clear_screen() -> crossterm::Result<()> {
        execute!(stdout(), terminal::Clear(terminal::ClearType::All))?;
        execute!(stdout(), crossterm::cursor::MoveTo(0, 0))
    }

    pub fn refresh_screen(&mut self) -> crossterm::Result<()> {
        self.cursor_controller.scroll(&self.editor_rows);
        queue!(
            self.editor_contents,
            crossterm::cursor::Hide,
            crossterm::cursor::MoveTo(0, 0)
        )?;
        self.draw_rows();
        self.draw_status_bar();
        self.draw_message_bar();
        let cursor_x = self.cursor_controller.render_x - self.cursor_controller.column_offset;
        let cursor_y = self.cursor_controller.cursor_y - self.cursor_controller.row_offset;
        queue!(
            self.editor_contents,
            crossterm::cursor::MoveTo(cursor_x as u16, cursor_y as u16),
            crossterm::cursor::Show
        )?;
        self.editor_contents.flush()
    }

    pub fn move_cursor(&mut self, direction: KeyCode) {
        self.cursor_controller
            .move_cursor(direction, &self.editor_rows);
    }

    pub fn page_up_down(&mut self, code: KeyCode) {
        match code {
            KeyCode::PageUp => self.cursor_controller.cursor_y = self.cursor_controller.row_offset,
            KeyCode::PageDown => {
                self.cursor_controller.cursor_y = self
                    .editor_rows
                    .number_of_row()
                    .min(self.win_size.1 + self.cursor_controller.row_offset - 1)
            }
            _ => unimplemented!(),
        }

        (0..self.win_size.1).for_each(|_| {
            self.move_cursor(if matches!(code, KeyCode::PageUp) {
                KeyCode::Up
            } else {
                KeyCode::Down
            })
        })
    }

    pub fn insert_char(&mut self, ch: char) {
        if self.cursor_controller.cursor_y == self.editor_rows.number_of_row() {
            self.editor_rows
                .insert_row(self.editor_rows.number_of_row(), String::new());
            self.dirty += 1;
        }
        self.editor_rows
            .get_editor_row_mut(self.cursor_controller.cursor_y)
            .insert_char(self.cursor_controller.cursor_x, ch);
        self.cursor_controller.cursor_x += 1;
        self.dirty += 1;
    }

    pub fn insert_newline(&mut self) {
        if self.cursor_controller.cursor_x == 0 {
            self.editor_rows
                .insert_row(self.cursor_controller.cursor_y, String::new());
        } else {
            let current_row = self
                .editor_rows
                .get_editor_row_mut(self.cursor_controller.cursor_y);
            let new_row_content = current_row.row_content[self.cursor_controller.cursor_x..].into();
            current_row
                .row_content
                .truncate(self.cursor_controller.cursor_x);
            EditorRows::render_row(current_row);
            self.editor_rows
                .insert_row(self.cursor_controller.cursor_y + 1, new_row_content);
        }
        self.cursor_controller.cursor_x = 0;
        self.cursor_controller.cursor_y += 1;
        self.dirty += 1;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_controller.cursor_y == self.editor_rows.number_of_row() {
            return;
        }

        if self.cursor_controller.cursor_y == 0 && self.cursor_controller.cursor_x == 0 {
            return;
        }

        let row = self
            .editor_rows
            .get_editor_row_mut(self.cursor_controller.cursor_y);
        if self.cursor_controller.cursor_x > 0 {
            row.delete_char(self.cursor_controller.cursor_x - 1);
            self.cursor_controller.cursor_x -= 1;
        } else {
            let previous_row_content = self
                .editor_rows
                .get_row(self.cursor_controller.cursor_y - 1);
            self.cursor_controller.cursor_x = previous_row_content.len();
            self.editor_rows
                .join_adjacent_rows(self.cursor_controller.cursor_y);
            self.cursor_controller.cursor_y -= 1;
        }
        self.dirty += 1;
    }

    pub fn save(&mut self) -> crossterm::Result<()> {
        self.editor_rows.save().map(|len| {
            self.status_message
                .set_message(format!("{} bytes written to disk", len));
            self.dirty = 0
        })
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty > 0
    }

    pub fn set_message(&mut self, message: String) {
        self.status_message.set_message(message)
    }

    fn draw_rows(&mut self) {
        let screen_row = self.win_size.1;
        let screen_column = self.win_size.0;

        for i in 0..screen_row {
            let file_row = i + self.cursor_controller.row_offset;
            if file_row >= self.editor_rows.number_of_row() {
                // ファイルの行数以上の行の描画
                if i == screen_row / 3 && self.editor_rows.number_of_row() == 0 {
                    self.draw_welcome();
                } else {
                    self.editor_contents.push('~');
                }
            } else {
                // ファイルコンテンツの描画
                let row = self.editor_rows.get_render(file_row);
                let column_offset = self.cursor_controller.column_offset;
                let len = row.len().saturating_sub(column_offset).min(screen_column);
                let start = if len == 0 { 0 } else { column_offset };
                self.editor_contents.push_str(&row[start..start + len]);
            }

            queue!(
                self.editor_contents,
                terminal::Clear(terminal::ClearType::UntilNewLine)
            )
            .unwrap();
            self.editor_contents.push_str("\r\n");
        }
    }

    fn draw_welcome(&mut self) {
        let screen_column = self.win_size.0;
        let mut welcome = format!("Pound Editor --- Version {}", "1.0.0");
        if welcome.len() > screen_column {
            welcome.truncate(screen_column);
        }
        let mut padding = (screen_column - welcome.len()) / 2;
        if padding != 0 {
            self.editor_contents.push('~');
            padding -= 1;
        }
        (0..padding).for_each(|_| self.editor_contents.push(' '));
        self.editor_contents.push_str(&welcome);
    }

    fn draw_status_bar(&mut self) {
        self.editor_contents
            .push_str(&style::Attribute::Reverse.to_string());

        let info = format!(
            "{} {} -- {} lines",
            self.editor_rows.filename(),
            if self.dirty > 0 { "(modified)" } else { "" },
            self.editor_rows.number_of_row()
        );
        let info_len = info.len().min(self.win_size.0);
        let line_info = format!(
            "{}/{}",
            self.cursor_controller.cursor_y + 1,
            self.editor_rows.number_of_row()
        );

        self.editor_contents.push_str(&info[..info_len]);

        for i in info_len..self.win_size.0 {
            if self.win_size.0 - i == line_info.len() {
                self.editor_contents.push_str(&line_info);
                break;
            } else {
                self.editor_contents.push(' ');
            }
        }

        self.editor_contents
            .push_str(&style::Attribute::Reset.to_string());
        self.editor_contents.push_str("\r\n");
    }

    fn draw_message_bar(&mut self) {
        queue!(
            self.editor_contents,
            terminal::Clear(terminal::ClearType::UntilNewLine)
        )
        .unwrap();
        if let Some(msg) = self.status_message.message() {
            self.editor_contents
                .push_str(&msg[..msg.len().min(self.win_size.0)]);
        }
    }
}

struct EditorContents {
    content: String,
}

impl EditorContents {
    fn new() -> Self {
        Self {
            content: String::new(),
        }
    }

    fn push(&mut self, ch: char) {
        self.content.push(ch)
    }

    fn push_str(&mut self, string: &str) {
        self.content.push_str(string)
    }
}

impl io::Write for EditorContents {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => {
                self.content.push_str(s);
                Ok(s.len())
            }
            Err(_) => Err(io::ErrorKind::WriteZero.into()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let out = write!(stdout(), "{}", self.content);
        stdout().flush()?;
        self.content.clear();
        out
    }
}
