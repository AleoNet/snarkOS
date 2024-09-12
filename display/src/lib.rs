// Copyright 2024 Aleo Network Foundation
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:

// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![forbid(unsafe_code)]

mod pages;
use pages::*;

mod tabs;
use tabs::Tabs;

use snarkos_node::Node;
use snarkvm::prelude::Network;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Tabs as TabsTui},
    Frame,
    Terminal,
};
use std::{
    io,
    thread,
    time::{Duration, Instant},
};
use tokio::sync::mpsc::Receiver;

pub struct Display<N: Network> {
    /// An instance of the node.
    node: Node<N>,
    /// The tick rate of the display.
    tick_rate: Duration,
    /// The state of the tabs.
    tabs: Tabs,
    /// The logs tab.
    logs: Logs,
}

impl<N: Network> Display<N> {
    /// Initializes a new display.
    pub fn start(node: Node<N>, log_receiver: Receiver<Vec<u8>>) -> Result<()> {
        // Initialize the display.
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Initialize the display.
        let mut display = Self {
            node,
            tick_rate: Duration::from_secs(1),
            tabs: Tabs::new(PAGES.to_vec()),
            logs: Logs::new(log_receiver),
        };

        // Render the display.
        let res = display.render(&mut terminal);

        // Terminate the display.
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
        terminal.show_cursor()?;

        // Exit.
        if let Err(err) = res {
            println!("{err:?}")
        }

        Ok(())
    }
}

impl<N: Network> Display<N> {
    /// Renders the display.
    fn render<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        let mut last_tick = Instant::now();
        loop {
            terminal.draw(|f| self.draw(f))?;

            // Set the timeout duration.
            let timeout = self.tick_rate.checked_sub(last_tick.elapsed()).unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Esc => {
                            // // TODO (howardwu): @ljedrz to implement a wrapping scope for Display within Node/Server.
                            // #[allow(unused_must_use)]
                            // {
                            //     self.node.shut_down();
                            // }
                            return Ok(());
                        }
                        KeyCode::Left => self.tabs.previous(),
                        KeyCode::Right => self.tabs.next(),
                        _ => {}
                    }
                }
            }

            if last_tick.elapsed() >= self.tick_rate {
                thread::sleep(Duration::from_millis(50));
                last_tick = Instant::now();
            }
        }
    }

    /// Draws the display.
    fn draw(&mut self, f: &mut Frame) {
        /* Layout */

        // Initialize the layout of the page.
        let chunks = Layout::default()
            .margin(1)
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
            .split(f.size());

        /* Tabs */

        // Initialize the tabs.
        let block = Block::default().style(Style::default().bg(Color::Black).fg(Color::White));
        f.render_widget(block, f.size());
        let titles = self
            .tabs
            .titles
            .iter()
            .map(|t| {
                let (first, rest) = t.split_at(1);
                Line::from(vec![
                    Span::styled(first, Style::default().fg(Color::Yellow)),
                    Span::styled(rest, Style::default().fg(Color::Green)),
                ])
            })
            .collect();
        let tabs = TabsTui::new(titles)
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

        /* Pages */

        // Initialize the page.
        match self.tabs.index {
            0 => Overview.draw(f, chunks[1], &self.node),
            1 => self.logs.draw(f, chunks[1]),
            _ => unreachable!(),
        };
    }
}
