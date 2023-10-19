// Copyright (C) 2019-2023 Aleo Systems Inc.
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

fn strip_newlines(buf: &[u8]) -> Vec<u8> {
    // Remove all newlines and then append one if the buffer ends with one;
    // it should always be the case, but it's cheap to make extra sure.
    buf.iter()
        .copied()
        .filter(|&b| b != b'\n')
        .chain(if matches!(buf.last(), Some(b'\n')) { Some(b'\n') } else { None })
        .collect()
}

pub struct WriterWrapper<W: io::Write>(pub W);

impl<W: io::Write> io::Write for WriterWrapper<W> {
    /// Writes the given buffer into the log writer.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let sanitized = strip_newlines(buf);
        // Force all the bytes to be written at once, otherwise
        // buffer accounting could fail, resulting in random
        // artifacts being written additionally.
        self.0.write_all(&sanitized)?;
        // Report the unsanitized size as the number of bytes
        // written for the same reason as above.
        Ok(buf.len())
    }

    /// Flushes the log writer (no-op).
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
