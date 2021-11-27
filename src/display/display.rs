// Copyright (C) 2019-2021 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::{
    display::{logs::Logs, overview::Overview, peers_status::PeersStatus},
    initialize_logger,
    network::Server,
    Environment,
};
use snarkvm::dpc::Network;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io,
    thread,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Tabs},
    Frame,
    Terminal,
};

pub struct TabsState<'a> {
    pub titles: Vec<&'a str>,
    pub index: usize,
}

impl<'a> TabsState<'a> {
    pub fn new(titles: Vec<&'a str>) -> TabsState {
        TabsState { titles, index: 0 }
    }

    pub fn next(&mut self) {
        self.index = (self.index + 1) % self.titles.len();
    }

    pub fn previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.titles.len() - 1;
        }
    }
}

pub(crate) struct Display<'a, N: Network, E: Environment> {
    server: Server<N, E>,
    tabs: TabsState<'a>,
    tick_rate: Duration,
    logs: Logs,
    peers_status: PeersStatus,
}

impl<'a, N: Network, E: Environment> Display<'a, N, E> {
    pub fn start(server: Server<N, E>, verbosity: u8) -> Result<()> {
        // Initialize the log channel.
        let (log_sender, log_receiver) = mpsc::channel(1024);

        initialize_logger(verbosity, Some(log_sender));

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

        // Initialize the display.
        let mut display = Display::<'a, N, E> {
            server,
            tabs: TabsState::new(vec![" Overview ", " Logs ", " Peers "]),
            tick_rate: Duration::from_secs(1),
            logs: Logs::new(log_receiver),
            peers_status: PeersStatus::new(),
        };

        let res = display.render(&mut terminal);

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
        terminal.show_cursor()?;

        if let Err(err) = res {
            println!("{:?}", err)
        }
        Ok(())
    }

    fn render<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        let mut last_tick = Instant::now();
        loop {
            terminal.draw(|f| self.draw(f))?;

            let timeout = self
                .tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Esc => {
                            self.server.shut_down();
                            return Ok(());
                        }
                        KeyCode::Left => self.tabs.previous(),
                        KeyCode::Right => self.tabs.next(),
                        KeyCode::Up => self.peers_status.previous(),
                        KeyCode::Down => self.peers_status.next(),
                        _ => {}
                    }
                }
            }

            if last_tick.elapsed() >= self.tick_rate {
                self.heartbeat();
                last_tick = Instant::now();
            }
        }
    }

    fn heartbeat(&mut self) {
        self.peers_status.update_data(&self.server);
        thread::sleep(Duration::from_millis(50));
    }

    fn draw<B: Backend>(&mut self, f: &mut Frame<B>) {
        // Initialize the layout of the page.
        let chunks = Layout::default()
            .margin(1)
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
            .split(f.size());

        // Initialize the tabs.
        let block = Block::default().style(Style::default().bg(Color::Black).fg(Color::White));
        f.render_widget(block, f.size());
        let titles = self
            .tabs
            .titles
            .iter()
            .map(|t| {
                let (first, rest) = t.split_at(1);
                Spans::from(vec![
                    Span::styled(first, Style::default().fg(Color::Yellow)),
                    Span::styled(rest, Style::default().fg(Color::Green)),
                ])
            })
            .collect();
        let tabs = Tabs::new(titles)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Welcome to Aleo.")
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .select(self.tabs.index)
            .style(Style::default().fg(Color::Cyan))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::White));
        f.render_widget(tabs, chunks[0]);

        // Initialize the page.
        match self.tabs.index {
            0 => Overview.draw(f, chunks[1]),
            1 => self.logs.draw(f, chunks[1]),
            2 => self.peers_status.draw(f, chunks[1]),
            _ => unreachable!(),
        };
    }
}
