use std::{env, fs, path::PathBuf};

const TAB_STOP: usize = 8;

pub struct EditorRows {
    row_contents: Vec<Row>,
    filename: Option<PathBuf>,
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

    fn render_row(row: &mut Row) {
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

pub struct Row {
    pub row_content: Box<str>,
    render: String,
}

impl Row {
    fn new(row_content: Box<str>, render: String) -> Self {
        Self {
            row_content,
            render,
        }
    }
}
