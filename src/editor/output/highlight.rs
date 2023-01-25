use crossterm::{
    queue,
    style::{Color, SetForegroundColor},
};

use super::{row::Row, EditorContents};

#[derive(Copy, Clone)]
pub enum HighlightType {
    Normal,
    Number,
    SearchMatch,
    String,
    CharLiteral,
    Comment,
    Other(Color),
}

pub trait SyntaxHighlight {
    fn syntax_color(&self, highlight_type: &HighlightType) -> Color;
    fn update_syntax(&self, at: usize, editor_rows: &mut Vec<Row>);
    fn extensions(&self) -> &[&str];
    fn file_type(&self) -> &str;
    fn comment_start(&self) -> &str;

    fn color_row(&self, render: &str, highlight: &[HighlightType], out: &mut EditorContents) {
        let mut current_color = self.syntax_color(&HighlightType::Normal);
        render.chars().enumerate().for_each(|(i, c)| {
            let color = self.syntax_color(&highlight[i]);
            if color != current_color {
                current_color = color;
                let _ = queue!(out, SetForegroundColor(color));
            }
            out.push(c);
        });
        let _ = queue!(out, SetForegroundColor(Color::Reset));
    }

    fn is_separator(&self, c: char) -> bool {
        c.is_whitespace()
            || [
                ',', '.', '(', ')', '+', '-', '/', '*', '=', '~', '%', '<', '>', '"', '\'', ';',
                '&',
            ]
            .contains(&c)
    }
}

#[macro_export]
macro_rules! syntax_struct {
    (
			struct $Name:ident {
                extensions:$ext:expr,
                file_type:$type:expr,
                comment_start:$start:expr,
                keywords: {
                    $([$color:expr; $($words:expr),*]),*
                }
            }
		) => {
        use crate::editor::output::highlight::HighlightType;
        use crate::editor::output::row::Row;

        struct $Name {
            extensions: &'static [&'static str],
            file_type: &'static str,
            comment_start: &'static str,
        }

        impl $Name {
            fn new() -> Self {
                $ (
                    let color = $color;
                    let keywords = vec!($($words),*);
                )*
                Self {
                    extensions: &$ext,
                    file_type: $type,
                    comment_start: $start,
                }
            }
        }

        impl SyntaxHighlight for $Name {
            fn extensions(&self) -> &[&str] {
                self.extensions
            }

            fn file_type(&self) -> &str {
                self.file_type
            }

            fn comment_start(&self) -> &str {
                self.comment_start
            }

            fn syntax_color(&self, highlight_type: &HighlightType) -> Color {
                match highlight_type {
                    HighlightType::Normal => Color::Reset,
                    HighlightType::Number => Color::Cyan,
                    HighlightType::SearchMatch => Color::Blue,
                    HighlightType::String => Color::Green,
                    HighlightType::CharLiteral => Color::DarkGreen,
                    HighlightType::Comment => Color::DarkGrey,
                    HighlightType::Other(color) => *color,
                }
            }

            fn update_syntax(&self, at: usize, editor_rows: &mut Vec<Row>) {
                let current_row = &mut editor_rows[at];
                macro_rules! add {
                    ($h:expr) => {
                        current_row.highlight.push($h)
                    };
                }

                current_row.highlight = Vec::with_capacity(current_row.render.len());
                let render = current_row.render.as_bytes();
                let mut i = 0;
                let mut previous_separator = true;
                let mut in_string: Option<char> = None;
                let comment_start = self.comment_start().as_bytes();

                while i < render.len() {
                    let c = render[i] as char;
                    let previous_highlight = if i > 0 {
                        current_row.highlight[i - 1]
                    } else {
                        HighlightType::Normal
                    };

                    if in_string.is_none() && !comment_start.is_empty() {
                        let end = i + comment_start.len();
                        if render[i..end.min(render.len())] == *comment_start {
                            (i..render.len()).for_each(|_| add!(HighlightType::Comment));
                            break;
                        }
                    }

                    if let Some(val) = in_string {
                        add! {
                            if val == '"' {HighlightType::String} else {HighlightType::CharLiteral}
                        }

                        if c == '\\' && i + 1 < render.len() {
                            add! {
                                if val == '"' {HighlightType::String} else {HighlightType::CharLiteral}
                            }
                            i += 2;
                            continue;
                        }

                        if val == c {
                            in_string = None;
                        }
                        i += 1;
                        previous_separator = true;
                        continue;
                    } else if c == '"' || c == '\'' {
                        in_string = Some(c);
                        add! {
                            if c == '"' {HighlightType::String} else {HighlightType::CharLiteral}
                        }
                        i += 1;
                        continue;
                    }

                    let is_number = c.is_digit(10)
                        && (previous_separator
                            || matches!(previous_highlight, HighlightType::Number));
                    let is_decimal_point =
                        c == '.' && matches!(previous_highlight, HighlightType::Number);

                    if is_number || is_decimal_point {
                        add!(HighlightType::Number);
                        i += 1;
                        previous_separator = false;
                        continue;
                    }

                    if previous_separator {
                        $ (
                            $ (
                                let end = i + $words.len();
                                let is_end_or_sep = render
                                    .get(end)
                                    .map(|c| self.is_separator(*c as char))
                                    .unwrap_or(end == render.len());
                                if is_end_or_sep && render[i..end] == *$words.as_bytes() {
                                    (i..i + $words.len()).for_each(|_| add!(HighlightType::Other($color)));
                                    i += $words.len();
                                    previous_separator = false;
                                    continue;
                                }

                            )*
                        )*
                    }

                    add!(HighlightType::Normal);
                    previous_separator = self.is_separator(c);
                    i += 1;
                }

                assert_eq!(current_row.render.len(), current_row.highlight.len())
            }
        }
    };
}
