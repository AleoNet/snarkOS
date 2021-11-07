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
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{canvas::Canvas, Block, Borders},
    Frame,
};

pub(super) struct Overview;

impl Overview {
    pub(super) fn draw<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        // Initialize the layout of the page.
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(8),
                    Constraint::Length(10),
                    Constraint::Percentage(90),
                    Constraint::Length(3),
                ]
                .as_ref(),
            )
            .split(area);

        let canvas = Canvas::default()
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .paint(|ctx| {
                // ctx.draw(&ball);
            });
        f.render_widget(canvas, chunks[0]);

        let canvas = Canvas::default()
            .block(Block::default().borders(Borders::ALL).title("Peers"))
            .paint(|ctx| {
                // ctx.draw(&ball);
            });
        f.render_widget(canvas, chunks[1]);

        let canvas = Canvas::default()
            .block(Block::default().borders(Borders::ALL).title("Logs"))
            .paint(|ctx| {
                // ctx.draw(&ball);
            });
        f.render_widget(canvas, chunks[2]);

        let canvas = Canvas::default()
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .paint(|ctx| {
                ctx.print(0f64, 0f64, "Press ESC to quit", Color::White);
            });
        f.render_widget(canvas, chunks[3]);
    }
}
