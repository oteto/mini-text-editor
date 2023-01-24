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
}

pub trait SyntaxHighlight {
    fn syntax_color(&self, highlight_type: &HighlightType) -> Color;
    fn update_syntax(&self, at: usize, editor_rows: &mut Vec<Row>);
    fn extensions(&self) -> &[&str];

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
            ]
            .contains(&c)
    }
}

#[macro_export]
macro_rules! syntax_struct {
    (
			struct $Name:ident {
                extensions:$ext:expr
            }
		) => {
        use crate::editor::output::highlight::HighlightType;
        use crate::editor::output::row::Row;

        struct $Name {
            extensions: &'static [&'static str],
        }

        impl $Name {
            fn new() -> Self {
                Self { extensions: &$ext }
            }
        }

        impl SyntaxHighlight for $Name {
            fn extensions(&self) -> &[&str] {
                self.extensions
            }

            fn syntax_color(&self, highlight_type: &HighlightType) -> Color {
                match highlight_type {
                    HighlightType::Normal => Color::Reset,
                    HighlightType::Number => Color::Cyan,
                    HighlightType::SearchMatch => Color::Blue,
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

                while i < render.len() {
                    let c = render[i] as char;
                    let previous_highlight = if i > 0 {
                        current_row.highlight[i - 1]
                    } else {
                        HighlightType::Normal
                    };

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
                    } else {
                        add!(HighlightType::Normal);
                    }
                    previous_separator = self.is_separator(c);
                    i += 1;
                }

                assert_eq!(current_row.render.len(), current_row.highlight.len())
            }
        }
    };
}
