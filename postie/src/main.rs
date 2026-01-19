/*
Copyright (C) 2025 Postem

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as
published by the Free Software Foundation, either version 3 of the
License, or (at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
use postem::PostemError;
use postem::client::ConnectionType;
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, Event, KeyCode, KeyModifiers},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::{Rect, Size},
};
use std::{error::Error, fs, io};

use crate::{
    app::{App, AppState},
    interactions::AppInteractions,
};
use crate::{
    interactions::InputType,
    ui::{ui, wait_pop_up},
};

mod app;
mod interactions;
mod theme;
mod ui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // create app and try to init client if fail error will stop program
    let args: Vec<String> = std::env::args().collect();
    println!("Trying to connect to antnet...");
    let mut connection_type = ConnectionType::Antnet;
    if args.len() > 1 {
        if &args[1] == "local" {
            connection_type = ConnectionType::Local;
        }
    }
    let mut app = App::create(connection_type).await?;
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen,)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let _res = run_app(&mut terminal, &mut app).await?;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    )?;
    terminal.show_cursor()?;

    Ok(())
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    let interactions = AppInteractions::new();
    let previous_buffer = terminal.draw(|f| ui(f, app, &interactions))?.buffer.clone();
    loop {
        let previous_buffer = terminal.draw(|f| ui(f, app, &interactions))?.buffer.clone();
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                // Skip events that are not KeyEvenKind::Press
                continue;
            }
            let mut error = None;
            let theme = app.theme();
            let input = InputType::derive(app, app.app_state(), key);
            if let Some(action) = interactions.get(app.app_state(), input) {
                match action.execute_or_async(app) {
                    Ok(Some(async_function)) => {
                        match wait_pop_up(
                            terminal,
                            previous_buffer,
                            async_function,
                            "How should I know what message to say...",
                            theme,
                        )
                        .await
                        {
                            Err(e) => error = Some(e),
                            _ => (),
                        }
                    }
                    Ok(None) => (),
                    Err(e) => error = Some(e),
                }
            }
            if let Some(error) = error {
                app.set_error(error);
            }
        }
        if app.app_state() == AppState::Quit {
            break;
        }
    }
    Ok(())
}
