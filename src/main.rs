mod editor;

use crossterm::terminal;
use editor::Editor;

struct CleanUp;

impl Drop for CleanUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not turn off raw mode");
    }
}

fn main() -> crossterm::Result<()> {
    let editor = Editor::new();
    editor.init()?;

    while editor.run()? {}

    Ok(())
}
