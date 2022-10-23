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

use std::io;
use tokio::sync::mpsc;

pub enum LogWriter {
    /// Writes to stdout.
    Stdout(io::Stdout),
    /// Writes to a channel.
    Sender(mpsc::Sender<Vec<u8>>),
}

impl LogWriter {
    /// Initialize a new log writer.
    pub fn new(log_sender: &Option<mpsc::Sender<Vec<u8>>>) -> Self {
        if let Some(sender) = log_sender { Self::Sender(sender.clone()) } else { Self::Stdout(io::stdout()) }
    }
}

impl io::Write for LogWriter {
    /// Writes the given buffer into the log writer.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Stdout(stdout) => stdout.write(buf),
            Self::Sender(sender) => {
                let log = buf.to_vec();
                let _ = sender.try_send(log);
                Ok(buf.len())
            }
        }
    }

    /// Flushes the log writer (no-op).
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
