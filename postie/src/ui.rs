/*
Copyright (C) 2025-2026 Postem

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
use ratatui::buffer::Buffer;
use ratatui::widgets::Widget;
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    text::{Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};
use std::io;
use tokio::time::{Duration, sleep};

use crate::theme::Theme;

/// pops up a wating popup while awaiting a future
pub async fn wait_pop_up<B: Backend>(
    // frame: &mut Frame<'_>,
    terminal: &mut Terminal<B>,
    previous_buffer: Buffer,
    future: impl Future<Output = Result<(), PostemError>>,
    message: &str,
    theme: Theme,
) -> Result<(), PostemError> {
    let mut count = 0;
    let animate = async {
        let mut antimation = Antimation::new();
        while count < 1200 {
            let result = terminal.draw(|frame| {
                frame.buffer_mut().merge(&previous_buffer);
                let area = frame.area();
                let pop_up_rect = area.inner(Margin::new(area.width / 4, area.height / 4));
                Clear.render(pop_up_rect, frame.buffer_mut());
                let pop_up_block = Block::default()
                    .title("Working...")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Thick)
                    .style(theme.header_style());
                let ant_frame = antimation.next_frame();
                let pop_up_text = Paragraph::new(Text::styled(
                    format!("{message}\n {ant_frame}"),
                    Style::default(),
                ))
                .wrap(Wrap { trim: false })
                .block(pop_up_block);
                frame.render_widget(pop_up_text, pop_up_rect);
            });
            count += 1;
            sleep(Duration::from_millis(500)).await;
            // try directly calling Error pop up rather than returning Error
            // match result {
            //     Err(_) => return Err::<(), PostemError>(PostemError::ClientConnectionError),
            //     _ => (),
            // }
        }
        Err(PostemError::ClientConnectionError)
    };
    tokio::select! {
        e = animate => { e }
        f = future => { f }
    }?;
    Ok(())
}
pub struct Antimation {
    count: usize,
}
impl Antimation {
    fn new() -> Antimation {
        Antimation { count: 0 }
    }

    fn next_frame(&mut self) -> String {
        let frame = if self.count == 0 {
            "o o    \n  \\\\\n  (\"\")\n  >||<\n   /\\".to_string()
        } else if self.count == 2 {
            "   o o\n    //\n  (\"\")\n  >||<\n   /\\".to_string()
        } else {
            "  oo  \n   ||  \n  (\'\')\n  >||<  \n   /\\  ".to_string()
        };
        if self.count >= 3 {
            self.count = 0
        } else {
            self.count += 1;
        }
        frame
    }
}
