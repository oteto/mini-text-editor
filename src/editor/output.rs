#![allow(unused)]

mod cursor;
mod highlight;
mod row;
mod search;
mod status;

use std::io::{self, stdout, Write};
use std::path::PathBuf;

use crossterm::style::*;
use crossterm::{event::KeyCode, execute, queue, style, terminal};

use crate::{prompt, syntax_struct};

use self::highlight::SyntaxHighlight;
use self::search::{SearchDirection, SearchIndex};
use self::{cursor::CursorController, row::EditorRows, status::StatusMessage};

syntax_struct! {
    struct RustHighlight {
        extensions: ["rs"],
        file_type: "rust",
        comment_start: "//",
        keywords : {
            [Color::Red;
                "mod","unsafe","extern","crate","use","type","struct","enum","union","const","static",
                "mut","let","if","else","impl","trait","for","fn","self","Self", "while", "true","false",
                "in","continue","break","loop","match"
            ],
            [Color::Reset; "isize","i8","i16","i32","i64","usize","u8","u16","u32","u64","f32","f64",
                "char","str","bool"
            ]
        }
    }
}

pub struct Output {
    win_size: (usize, usize),
    editor_contents: EditorContents,
    cursor_controller: CursorController,
    editor_rows: EditorRows,
    status_message: StatusMessage,
    dirty: u64,
    search_index: SearchIndex,
    syntax_highlight: Option<Box<dyn SyntaxHighlight>>,
}

impl Output {
    pub fn new() -> Self {
        let win_size = terminal::size()
            .map(|(x, y)| (x as usize, y as usize - 2))
            .unwrap();
        let mut syntax_highlight = None;
        Self {
            win_size,
            editor_contents: EditorContents::new(),
            cursor_controller: CursorController::new(win_size),
            editor_rows: EditorRows::new(&mut syntax_highlight),
            status_message: StatusMessage::new(
                "HELP: Ctrl-S = Save | Ctrl-Q = Quit | Ctrl-F = Find".into(),
            ),
            dirty: 0,
            search_index: SearchIndex::new(),
            syntax_highlight,
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

        if let Some(it) = self.syntax_highlight.as_ref() {
            it.update_syntax(
                self.cursor_controller.cursor_y,
                &mut self.editor_rows.row_contents,
            )
        }

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

            if let Some(it) = self.syntax_highlight.as_ref() {
                it.update_syntax(
                    self.cursor_controller.cursor_y,
                    &mut self.editor_rows.row_contents,
                );
                it.update_syntax(
                    self.cursor_controller.cursor_y + 1,
                    &mut self.editor_rows.row_contents,
                )
            }
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
        if let Some(it) = self.syntax_highlight.as_ref() {
            it.update_syntax(
                self.cursor_controller.cursor_y,
                &mut self.editor_rows.row_contents,
            );
        }
        self.dirty += 1;
    }

    pub fn save(&mut self) -> crossterm::Result<()> {
        if matches!(self.editor_rows.filename, None) {
            let prompt = prompt!(self, "Save as : {}").map(|it| it.into());
            if let None = prompt {
                self.set_message("Save Aborted".into());
                return Ok(());
            }

            prompt
                .as_ref()
                .and_then(|path: &PathBuf| path.extension())
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    Output::select_syntax(ext).map(|syntax| {
                        let highlight = self.syntax_highlight.insert(syntax);
                        for i in 0..self.editor_rows.number_of_row() {
                            highlight.update_syntax(i, &mut self.editor_rows.row_contents);
                        }
                    })
                });

            self.editor_rows.filename = prompt;
        }

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

    pub fn find(&mut self) -> io::Result<()> {
        let cursor_controller = self.cursor_controller;
        if prompt!(
            self,
            "Search: {} (Use ESC / Arrows / Enter)",
            callback = Output::find_callback
        )
        .is_none()
        {
            self.cursor_controller = cursor_controller;
        }
        Ok(())
    }

    pub fn select_syntax(extension: &str) -> Option<Box<dyn SyntaxHighlight>> {
        let list: Vec<Box<dyn SyntaxHighlight>> = vec![Box::new(RustHighlight::new())];
        list.into_iter()
            .find(|it| it.extensions().contains(&extension))
    }

    fn find_callback(output: &mut Output, keyword: &str, key_code: KeyCode) {
        if let Some((index, highlight)) = output.search_index.previous_highlight.take() {
            output.editor_rows.get_editor_row_mut(index).highlight = highlight;
        }

        match key_code {
            KeyCode::Esc | KeyCode::Enter => {
                output.search_index.reset();
            }
            _ => {
                output.search_index.y_direction = None;
                output.search_index.x_direction = None;
                match key_code {
                    KeyCode::Down => {
                        output.search_index.y_direction = SearchDirection::Forward.into();
                    }
                    KeyCode::Up => {
                        output.search_index.y_direction = SearchDirection::Backward.into();
                    }
                    KeyCode::Left => {
                        output.search_index.x_direction = SearchDirection::Backward.into();
                    }
                    KeyCode::Right => {
                        output.search_index.x_direction = SearchDirection::Forward.into();
                    }
                    _ => {}
                }

                for i in 0..output.editor_rows.number_of_row() {
                    let row_index = match output.search_index.y_direction.as_ref() {
                        None => {
                            if output.search_index.x_direction.is_none() {
                                output.search_index.y_index = i;
                            }
                            output.search_index.y_index
                        }
                        Some(dir) => {
                            if matches!(dir, SearchDirection::Forward) {
                                output.search_index.y_index + i + 1
                            } else {
                                let res = output.search_index.y_index.saturating_sub(i);
                                if res == 0 {
                                    break;
                                }
                                res - 1
                            }
                        }
                    };

                    if row_index > output.editor_rows.number_of_row() - 1 {
                        break;
                    }

                    let row = output.editor_rows.get_editor_row_mut(row_index);
                    let index = match output.search_index.x_direction.as_ref() {
                        None => row.find(&keyword),
                        Some(dir) => {
                            let index = if matches!(dir, SearchDirection::Forward) {
                                let start = row.len().min(output.search_index.x_index + 1);
                                row.render[start..]
                                    .find(&keyword)
                                    .map(|index| index + start)
                            } else {
                                row.render[..output.search_index.x_index].rfind(&keyword)
                            };
                            if index.is_none() {
                                break;
                            }
                            index
                        }
                    };

                    if let Some(index) = index {
                        output.search_index.previous_highlight =
                            Some((row_index, row.highlight.clone()));

                        (index..index + keyword.len()).for_each(|index| {
                            row.highlight[index] = HighlightType::SearchMatch;
                        });

                        output.cursor_controller.cursor_y = row_index;
                        output.search_index.y_index = row_index;
                        output.search_index.x_index = index;
                        output.cursor_controller.cursor_x = row.get_row_content_x(index);
                        output.cursor_controller.row_offset = output.editor_rows.number_of_row();
                        break;
                    }
                }
            }
        }
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
                let row = self.editor_rows.get_editor_row(file_row);
                let render = &row.render;
                let column_offset = self.cursor_controller.column_offset;
                let len = row.len().saturating_sub(column_offset).min(screen_column);
                let start = if len == 0 { 0 } else { column_offset };

                self.syntax_highlight
                    .as_ref()
                    .map(|syntax_highlight| {
                        syntax_highlight.color_row(
                            &render[start..start + len],
                            &row.highlight[start..start + len],
                            &mut self.editor_contents,
                        )
                    })
                    .unwrap_or_else(|| self.editor_contents.push_str(&render[start..start + len]));
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
            "{} | {}/{}",
            self.syntax_highlight
                .as_ref()
                .map(|highlight| highlight.file_type())
                .unwrap_or("no ft"),
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

pub struct EditorContents {
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
