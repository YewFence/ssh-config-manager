use std::path::Path;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::DefaultTerminal;

use self::app::{AppSignal, TuiApp};

mod app;
mod editor;
mod fields;
mod render;

pub fn run(config_path: &Path) -> Result<()> {
    let mut app = TuiApp::load(config_path.to_path_buf())?;
    let mut terminal = ratatui::try_init()?;
    let result = run_app(&mut terminal, &mut app);
    ratatui::try_restore()?;
    result
}

fn run_app(terminal: &mut DefaultTerminal, app: &mut TuiApp) -> Result<()> {
    loop {
        terminal.draw(|frame| render::render(frame, app))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.kind != KeyEventKind::Press {
            continue;
        }

        if app.handle_key(key)? == AppSignal::Quit {
            break;
        }
    }

    Ok(())
}
