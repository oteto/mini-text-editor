use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
};

const TAB_STOP: usize = 8;

pub struct EditorRows {
    row_contents: Vec<Row>,
    pub filename: Option<PathBuf>,
}

impl EditorRows {
    pub fn new() -> Self {
        let mut arg = env::args();

        match arg.nth(1) {
            None => Self {
                row_contents: Vec::new(),
                filename: None,
            },
            Some(file) => Self::from_file(file.into()),
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

    fn from_file(file: PathBuf) -> Self {
        let file_contents = fs::read_to_string(&file).expect("Unable to read file");
        Self {
            filename: Some(file),
            row_contents: file_contents
                .lines()
                .map(|it| {
                    let mut row = Row::new(it.into(), String::new());
                    Self::render_row(&mut row);
                    row
                })
                .collect(),
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
    render: String,
}

impl Row {
    fn new(row_content: String, render: String) -> Self {
        Self {
            row_content,
            render,
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
}
