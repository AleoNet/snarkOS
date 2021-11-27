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

use tui::{
    backend::Backend,
    layout::{Constraint, Layout, Rect},
    Frame,
};

use crate::{Environment, Server};
use snarkvm::dpc::Network;
use tui::{
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};

pub(super) type PeersStatus = StatefulTable;

pub struct StatefulTable {
    state: TableState,
    data: Vec<Vec<String>>,
}

impl StatefulTable {
    pub(super) fn new() -> StatefulTable {
        StatefulTable {
            state: TableState::default(),
            data: vec![],
        }
    }

    pub(super) fn update_data<N: Network, E: Environment>(&mut self, server: &Server<N, E>) {
        self.data = match server.peers_state_snapshot() {
            Ok(map) => map
                .iter()
                .enumerate()
                .map(|(i, (peer_ip, peer_state))| {
                    let peer_state = peer_state.as_ref().unwrap();
                    let mut row = vec![];
                    row.push(i.to_string());
                    row.push(peer_ip.to_string());
                    row.push(peer_state.0.to_string());
                    row.push(peer_state.1.to_string());
                    row
                })
                .collect::<Vec<Vec<String>>>(),
            Err(_) => self.data.clone(),
        }
    }

    pub(super) fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.data.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub(super) fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.data.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub(super) fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        // Initialize the layout of the page.
        let rects = Layout::default().constraints([Constraint::Percentage(100)].as_ref()).split(area);

        let header_cells = ["Index", "Peer IP", "Node Type", "Status"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::White)));
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let rows = self.data.iter().map(|item| {
            let cells = vec![
                Cell::from(item[0].as_str()).style(Style::default().fg(Color::Gray)),
                Cell::from(item[1].as_str()).style(Style::default().fg(Color::White)),
                Cell::from(item[2].as_str()).style(Style::default().fg(Color::White)),
                Cell::from(item[3].as_str()).style(Style::default().fg(Color::White)),
            ];
            Row::new(cells).bottom_margin(1)
        });

        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let t = Table::new(rows)
            .header(header)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(selected_style)
            .widths(&[
                Constraint::Length(10),
                Constraint::Percentage(50),
                Constraint::Length(9),
                Constraint::Max(10),
            ]);
        f.render_stateful_widget(t, rects[0], &mut self.state);
    }
}
