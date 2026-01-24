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
use ratatui::style::Stylize;
use ratatui::widgets::Widget;
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    text::{Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};
use std::cmp::min;
use tokio::time::{Duration, sleep};

use crate::App;
use crate::app::{AppState, CreateAddresseeState, PostPackageState};
use crate::interactions::AppInteractions;
use crate::theme::Theme;

pub fn ui(frame: &mut Frame, app: &mut App, interactions: &AppInteractions) {
    let area = frame.area();
    let mut status_text = String::new();
    let mut menu_options = interactions.get_menu_items(app.app_state());
    let ui_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Fill(1),
            Constraint::Length(5),
        ])
        .split(area);
    // uglify for windows terminals that don't support some charaters
    let title_borders = if std::env::consts::OS == "windows" {
        Borders::NONE
    } else {
        Borders::ALL
    };
    let title_block = Block::default()
        .borders(title_borders)
        .border_type(BorderType::QuadrantOutside)
        .style(app.theme.header_style())
        .bold();
    let mut url_style = app.theme.header_style();
    let title_text = Text::raw("Postie");
    let title = Paragraph::new(title_text).block(title_block);
    frame.render_widget(title, ui_chunks[0]);

    // modify based on current_view
    match &app.app_state() {
        AppState::Error => {
            if let Some(error) = app.error_text() {
                status_text = "Press (enter) to contunue or (q) to quit".to_string();
                let pop_up_rect = area.inner(Margin::new(area.width / 4, area.height / 4)); //centered_rect(60, 60, area);
                let navigation_text = "Press (enter) to contiune.";
                Clear.render(pop_up_rect, frame.buffer_mut());
                let pop_up_block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Thick)
                    .style(app.theme.text_style());
                frame.render_widget(pop_up_block, pop_up_rect);
                let pop_up_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([
                        Constraint::Percentage(100),
                        Constraint::Min(navigation_text.lines().count() as u16),
                    ])
                    .split(pop_up_rect);
                let pop_up_text =
                    Paragraph::new(Text::styled(format!("{error}"), Style::default()))
                        .wrap(Wrap { trim: false });
                frame.render_widget(pop_up_text, pop_up_chunks[0]);
                let navigation_text = Paragraph::new(
                    Text::styled(navigation_text, Style::default()).not_rapid_blink(),
                )
                .alignment(Alignment::Center);
                frame.render_widget(navigation_text, pop_up_chunks[1]);
            }
        }
        AppState::None => {
            status_text = "Press p to post a package, or c to create a new address".to_string();
            let pop_up_rect = area.inner(Margin::new(area.width / 4, area.height / 4));
            let navigation_text = interactions.get_menu_items(app.app_state()).join(", ");
            Clear.render(pop_up_rect, frame.buffer_mut());
            let pop_up_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .style(app.theme.text_style());
            frame.render_widget(pop_up_block, pop_up_rect);
            let pop_up_chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Percentage(100),
                    Constraint::Min(navigation_text.lines().count() as u16),
                ])
                .split(pop_up_rect);
            let pop_up_text = Paragraph::new(Text::styled(
                "Press p to post a pacakge or c to create a new address".to_string(),
                Style::default(),
            ))
            .wrap(Wrap { trim: false });
            frame.render_widget(pop_up_text, pop_up_chunks[0]);
            let navigation_text =
                Paragraph::new(Text::styled(navigation_text, Style::default()).not_rapid_blink())
                    .alignment(Alignment::Center);
            frame.render_widget(navigation_text, pop_up_chunks[1]);
        }
        AppState::CreateAddressee(create_address_state) => {
            let pop_up_rect = area.inner(Margin::new(area.width / 8, area.height / 5));
            let warning = "THIS IS EXPERIMENTAL SOFTWARE AND STORAGE COSTS MAY VARY WITHOUT WARNING SO DO NOT USE A WALLET WITH YOUR LIFE SAVINGS IN OR INDEED CONTAINING ANY AMOUNT YOU ARE NOT PREPARED TO LOSE IN ENTIRETY";
            Clear.render(pop_up_rect, frame.buffer_mut());
            let pop_up_block = Block::default()
                .title("Create new postem address")
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .style(app.theme.text_style());
            frame.render_widget(pop_up_block, pop_up_rect);
            let pop_up_chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Percentage(30),
                    Constraint::Percentage(40),
                ])
                .split(pop_up_rect);
            let mut name_block = Block::default()
                .title("Enter desired address: No semi-colons, commas or whitesapce");
            let mut key_block = Block::default()
                .title("Enter private key of funding wallet")
                .style(app.theme.text_style());
            let warning_block = Block::default().style(app.theme.header_style()).bold();
            match create_address_state {
                CreateAddresseeState::InputAddresseeName => {
                    status_text =
                        "Type to enter addressee name, press (enter) to proceed or (esc) to go leave"
                            .to_string();
                    name_block = name_block.clone().style(app.theme.inverted_text_style())
                }
                CreateAddresseeState::InputFundingWallet => {
                    status_text = "Type to enter key or use terminal emulator paste (enter) to proceed, (tab) to edit name or (esc) to leave".to_string();
                    key_block = key_block.clone().style(app.theme.inverted_text_style())
                }
            };
            let name_text = Paragraph::new(app.create_name_input().clone()).block(name_block);
            let key_text = Paragraph::new(app.create_key_input().clone()).block(key_block);
            let warning_text = Paragraph::new(warning)
                .wrap(Wrap { trim: false })
                .block(warning_block);
            frame.render_widget(name_text, pop_up_chunks[0]);
            frame.render_widget(warning_text, pop_up_chunks[1]);
            frame.render_widget(key_text, pop_up_chunks[2]);
        }
        AppState::PostPackage(post_package_state) => {
            let pop_up_rect = area.inner(Margin::new(area.width / 8, area.height / 5));
            let warning = "THIS IS EXPERIMENTAL SOFTWARE AND STORAGE COSTS MAY VARY WITHOUT WARNING SO DO NOT USE A WALLET WITH YOUR LIFE SAVINGS IN OR INDEED CONTAINING ANY AMOUNT YOU ARE NOT PREPARED TO LOSE IN ENTIRETY";
            Clear.render(pop_up_rect, frame.buffer_mut());
            let pop_up_block = Block::default()
                .title("Enter the recpients addressess (deliminate with ;) message text and private key of funding wallet")
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .style(app.theme.text_style());
            frame.render_widget(pop_up_block, pop_up_rect);
            let pop_up_chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                ])
                .split(pop_up_rect);
            let mut recipients_block = Block::default().title("Name").style(app.theme.text_style());
            let mut message_block = Block::default()
                .title("URL name: separate domains with full stops (.) leave blank for random URL");
            let mut key_block = Block::default()
                .title("Private key of funding wallet")
                .style(app.theme.text_style());
            let warning_block = Block::default().style(app.theme.header_style()).bold();
            match post_package_state {
                PostPackageState::InputRecipients => {
                    status_text =
                        "Type to enter bored name, press (enter) to proceed or (esc) to go leave"
                            .to_string();
                    recipients_block = recipients_block
                        .clone()
                        .style(app.theme.inverted_text_style())
                }
                PostPackageState::InputMessage => {
                    status_text =
                        "Type to url name, press (enter) to proceed or (esc) to go leave. Leave blank to have random url"
                            .to_string();
                    message_block = message_block.clone().style(app.theme.inverted_text_style())
                }
                PostPackageState::InputFundingWallet => {
                    status_text = "Type to enter key or use terminal emulator paste (enter) to proceed, (tab) to edit name or (esc) to leave".to_string();
                    key_block = key_block.clone().style(app.theme.inverted_text_style())
                }
            };
            let name_text =
                Paragraph::new(app.post_recipients_input().clone()).block(recipients_block);
            let url_name_text =
                Paragraph::new(app.post_message_input().clone()).block(message_block);
            let key_text = Paragraph::new(app.post_key_input().clone()).block(key_block);
            let warning_text = Paragraph::new(warning)
                .wrap(Wrap { trim: false })
                .block(warning_block);
            frame.render_widget(name_text, pop_up_chunks[0]);
            frame.render_widget(url_name_text, pop_up_chunks[1]);
            frame.render_widget(warning_text, pop_up_chunks[2]);
            frame.render_widget(key_text, pop_up_chunks[3]);
        }
        // View::DirectoryView(directory_index) => {
        //     let mut table_state = TableState::default().with_selected(*directory_index);
        //     let header = ["Bored name", "Home"]
        //         .into_iter()
        //         .map(Span::from)
        //         .collect::<Row>()
        //         .style(app.theme.text_style())
        //         .bold()
        //         .height(1);
        //     let directory_table = app.directory.as_table();
        //     let rows: Vec<Row> = directory_table
        //         .iter()
        //         .map(|r| Row::new(vec![r[0].clone(), r[1].clone()]).style(app.theme.text_style()))
        //         .collect();
        //     let pop_up_rect = area.inner(Margin::new(area.width / 8, area.height / 4));
        //     let pop_up_block = Block::default()
        //         .title("Diretory of boreds")
        //         .style(app.theme.text_style())
        //         .borders(Borders::ALL)
        //         .border_type(BorderType::Thick);
        //     let table = Table::new(rows, [Constraint::Fill(1), Constraint::Length(6)])
        //         .header(header)
        //         .row_highlight_style(app.theme.inverted_text_style())
        //         .block(pop_up_block);
        //     status_text =
        //         "Press up and down to select, (enter) to confirm selection, (ctrl + h) to set as home bored and (esc) to cancel"
        //             .to_string();
        //     Clear.render(pop_up_rect, frame.buffer_mut());
        //     frame.render_stateful_widget(table, pop_up_rect, &mut table_state);
        _ => (),
    }
    // setup status area
    let status_block = Block::default()
        .borders(title_borders)
        .border_type(BorderType::QuadrantOutside)
        .style(app.theme.header_style())
        .bold();
    // let status_rect = Rect::new(0, area.height - 5, area.width, 5);
    // status_text = format!("{:?}\n{}", app.status, status_text);
    let status = Paragraph::new(Text::styled(status_text, Style::default()))
        .wrap(Wrap { trim: false })
        .block(status_block);
    frame.render_widget(status, ui_chunks[2]);
    // status.render(status_rect, frame.buffer_mut());
    if app.menu_visible() {
        let menu_rect = Rect::new(
            safe_subtract_u16(area.width, 40),
            safe_subtract_u16(area.height, menu_options.len() as u16 + 2),
            min(40, area.width),
            min(menu_options.len() as u16 + 2, area.height),
        );
        let menu_text = menu_options.join("\n");
        let menu_block = Block::default()
            .title("Menu")
            .borders(Borders::ALL)
            .style(app.theme.dimmed_text_style());
        let menu = Paragraph::new(menu_text).bold().block(menu_block);
        Clear.render(menu_rect, frame.buffer_mut());
        frame.render_widget(menu, menu_rect);
    }
}

/// Returns 0 if subraction overflow
pub fn safe_subtract_u16(a: u16, b: u16) -> u16 {
    if (a as i32 - b as i32) < 0 { 0 } else { a - b }
}

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
            let _result = terminal
                .draw(|frame| {
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
                })
                .expect("Wait pop up animation frame failed to render");
            count += 1;
            sleep(Duration::from_millis(500)).await;
            // try directly calling Error pop up rather than returning Error
            // match result {
            //     Err(_) => return Err::<(), PostemError>(PostemError::ClientConnectionError),
            //     _ => (),
            // }
        }
        Err(PostemError::AntnetTimeOut)
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
