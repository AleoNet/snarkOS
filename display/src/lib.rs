// Copyright (C) 2019-2022 Aleo Systems Inc.
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

#![forbid(unsafe_code)]

mod log_writer;
use log_writer::*;

mod pages;
use pages::*;

mod tabs;
use tabs::Tabs;

use snarkos_node::Node;
use snarkvm::prelude::Network;

use anyhow::Result;
use colored::*;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
};
use std::{
    io,
    thread,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Tabs as TabsTui},
    Frame,
    Terminal,
};

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
    pub fn start(node: Node<N>, verbosity: u8, nodisplay: bool) -> Result<()> {
        // Initialize the logger.
        let log_receiver = Self::initialize_logger(verbosity, nodisplay);

        // If the display is not enabled, render the welcome message.
        if nodisplay {
            // Print the Aleo address.
            println!("🪪 Your Aleo address is {}.\n", node.address().to_string().bold());
            // Print the node type and network.
            println!("🧭 Starting {} on {}.\n", node.node_type().description().bold(), N::NAME.bold());
        }
        // If the display is enabled, render the display.
        else {
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
        }

        Ok(())
    }
}

impl<N: Network> Display<N> {
    /// Initializes the logger.
    fn initialize_logger(verbosity: u8, nodisplay: bool) -> mpsc::Receiver<Vec<u8>> {
        match verbosity {
            0 => std::env::set_var("RUST_LOG", "info"),
            1 => std::env::set_var("RUST_LOG", "debug"),
            2 | 3 => std::env::set_var("RUST_LOG", "trace"),
            _ => std::env::set_var("RUST_LOG", "info"),
        };

        // Filter out undesirable logs.
        let filter = EnvFilter::from_default_env()
            .add_directive("mio=off".parse().unwrap())
            .add_directive("tokio_util=off".parse().unwrap())
            .add_directive("hyper::proto::h1::conn=off".parse().unwrap())
            .add_directive("hyper::proto::h1::decode=off".parse().unwrap())
            .add_directive("hyper::proto::h1::io=off".parse().unwrap())
            .add_directive("hyper::proto::h1::role=off".parse().unwrap())
            .add_directive("jsonrpsee=off".parse().unwrap());

        // Initialize the log channel.
        let (log_sender, log_receiver) = mpsc::channel(1024);

        // Initialize the log sender.
        let log_sender = match nodisplay {
            true => None,
            false => Some(log_sender),
        };

        // Initialize tracing.
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(log_sender.is_none() && io::stdout().is_tty())
            .with_writer(move || LogWriter::new(&log_sender))
            .with_target(verbosity == 3)
            .try_init();

        log_receiver
    }

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
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>) {
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
                Spans::from(vec![
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

    /// Returns the welcome message as a string.
    pub fn welcome_message() -> String {
        use colored::*;

        let mut output = String::new();
        output += &r#"

         ╦╬╬╬╬╬╦
        ╬╬╬╬╬╬╬╬╬                    ▄▄▄▄        ▄▄▄
       ╬╬╬╬╬╬╬╬╬╬╬                  ▐▓▓▓▓▌       ▓▓▓
      ╬╬╬╬╬╬╬╬╬╬╬╬╬                ▐▓▓▓▓▓▓▌      ▓▓▓     ▄▄▄▄▄▄       ▄▄▄▄▄▄
     ╬╬╬╬╬╬╬╬╬╬╬╬╬╬╬              ▐▓▓▓  ▓▓▓▌     ▓▓▓   ▄▓▓▀▀▀▀▓▓▄   ▐▓▓▓▓▓▓▓▓▌
    ╬╬╬╬╬╬╬╜ ╙╬╬╬╬╬╬╬            ▐▓▓▓▌  ▐▓▓▓▌    ▓▓▓  ▐▓▓▓▄▄▄▄▓▓▓▌ ▐▓▓▓    ▓▓▓▌
   ╬╬╬╬╬╬╣     ╠╬╬╬╬╬╬           ▓▓▓▓▓▓▓▓▓▓▓▓    ▓▓▓  ▐▓▓▀▀▀▀▀▀▀▀▘ ▐▓▓▓    ▓▓▓▌
  ╬╬╬╬╬╬╣       ╠╬╬╬╬╬╬         ▓▓▓▓▌    ▐▓▓▓▓   ▓▓▓   ▀▓▓▄▄▄▄▓▓▀   ▐▓▓▓▓▓▓▓▓▌
 ╬╬╬╬╬╬╣         ╠╬╬╬╬╬╬       ▝▀▀▀▀      ▀▀▀▀▘  ▀▀▀     ▀▀▀▀▀▀       ▀▀▀▀▀▀
╚╬╬╬╬╬╩           ╩╬╬╬╬╩


"#
        .white()
        .bold();
        output += &"👋 Welcome to Aleo! We thank you for running a node and supporting privacy.\n".bold();
        output
    }
}
