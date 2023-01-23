use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
};

use super::highlight::{HighlightType, SyntaxHighlight};

const TAB_STOP: usize = 8;

pub struct EditorRows {
    pub row_contents: Vec<Row>,
    pub filename: Option<PathBuf>,
}

impl EditorRows {
    pub fn new(syntax_highlight: Option<&dyn SyntaxHighlight>) -> Self {
        let mut arg = env::args();

        match arg.nth(1) {
            None => Self {
                row_contents: Vec::new(),
                filename: None,
            },
            Some(file) => Self::from_file(file.into(), syntax_highlight),
        }
    }

    pub fn filename(&self) -> &str {
        self.filename
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("[No Name]")
    }

    pub fn get_editor_row(&self, at: usize) -> &Row {
        &self.row_contents[at]
    }

    pub fn number_of_row(&self) -> usize {
        self.row_contents.len()
    }

    pub fn get_row(&self, at: usize) -> &str {
        &self.row_contents[at].row_content
    }

    fn from_file(file: PathBuf, syntax_highlight: Option<&dyn SyntaxHighlight>) -> Self {
        let file_contents = fs::read_to_string(&file).expect("Unable to read file");
        let mut row_contents = Vec::new();
        file_contents.lines().enumerate().for_each(|(i, line)| {
            let mut row = Row::new(line.into(), String::new());
            Self::render_row(&mut row);
            row_contents.push(row);
            if let Some(it) = syntax_highlight {
                it.update_syntax(i, &mut row_contents)
            }
        });
        Self {
            filename: Some(file),
            row_contents,
        }
    }

    pub fn get_render(&self, at: usize) -> &String {
        &self.row_contents[at].render
    }

    pub fn insert_row(&mut self, at: usize, contents: String) {
        let mut new_row = Row::new(contents, String::new());
        EditorRows::render_row(&mut new_row);
        self.row_contents.insert(at, new_row);
    }

    pub fn get_editor_row_mut(&mut self, at: usize) -> &mut Row {
        &mut self.row_contents[at]
    }

    pub fn save(&self) -> io::Result<usize> {
        match &self.filename {
            None => Err(io::Error::new(
                io::ErrorKind::Other,
                "no file name specified",
            )),
            Some(name) => {
                let mut file = fs::OpenOptions::new().write(true).create(true).open(name)?;
                let contents: String = self
                    .row_contents
                    .iter()
                    .map(|it| it.row_content.as_str())
                    .collect::<Vec<&str>>()
                    .join("\n");
                file.set_len(contents.len() as u64)?;
                file.write_all(contents.as_bytes())?;
                Ok(contents.as_bytes().len())
            }
        }
    }

    pub fn join_adjacent_rows(&mut self, at: usize) {
        let current_row = self.row_contents.remove(at);
        let previous_row = self.get_editor_row_mut(at - 1);
        previous_row.row_content.push_str(&current_row.row_content);
        Self::render_row(previous_row);
    }

    pub fn render_row(row: &mut Row) {
        let mut index = 0;
        let capacity = row
            .row_content
            .chars()
            .fold(0, |acc, next| acc + if next == '\t' { TAB_STOP } else { 1 });
        row.render = String::with_capacity(capacity);
        row.row_content.chars().for_each(|c| {
            index += 1;
            if c == '\t' {
                row.render.push(' ');
                while index % TAB_STOP != 0 {
                    row.render.push(' ');
                    index += 1;
                }
            } else {
                row.render.push(c);
            }
        })
    }
}

#[derive(Default)]
pub struct Row {
    pub row_content: String,
    pub render: String,
    pub highlight: Vec<HighlightType>,
}

impl Row {
    fn new(row_content: String, render: String) -> Self {
        Self {
            row_content,
            render,
            highlight: Vec::new(),
        }
    }

    pub fn insert_char(&mut self, at: usize, ch: char) {
        self.row_content.insert(at, ch);
        EditorRows::render_row(self);
    }

    pub fn delete_char(&mut self, at: usize) {
        self.row_content.remove(at);
        EditorRows::render_row(self);
    }

    pub fn find(&self, keyword: &str) -> Option<usize> {
        self.render.find(keyword)
    }

    pub fn len(&self) -> usize {
        self.render.len()
    }

    pub fn get_row_content_x(&self, render_x: usize) -> usize {
        let mut current_render_x = 0;
        for (cursor_x, ch) in self.row_content.chars().enumerate() {
            if ch == '\t' {
                current_render_x += (TAB_STOP - 1) - (current_render_x % TAB_STOP);
            }
            current_render_x += 1;
            if current_render_x > render_x {
                return cursor_x;
            }
        }
        0
    }
}
