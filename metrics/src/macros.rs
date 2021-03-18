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

#[allow(unused_imports)]
use crate::Metrics;

#[macro_export]
macro_rules! connected_peers_inc {
    // Default case - increments the counter.
    () => {
        Metrics::connected_peers_inc()
    };
    // Boolean case - increments the counter if `$opt = true`, and passes the boolean through.
    ($opt:expr) => {{
        let boolean: bool = $opt; // Force types to be boolean
        if boolean {
            connected_peers_inc!()
        }
        boolean
    }};
}

#[macro_export]
macro_rules! connected_peers_dec {
    // Default case - decrements the counter.
    () => {
        Metrics::connected_peers_dec()
    };
    // Boolean case - decrements the counter if `$opt = true`, and passes the boolean through.
    ($opt:expr) => {{
        let boolean: bool = $opt; // Force types to be boolean
        if boolean {
            connected_peers_dec!()
        }
        boolean
    }};
}

#[cfg(test)]
mod tests {
    use crate::Metrics;
    use snarkvm_derives::test_with_metrics;

    use serial_test::serial;

    #[allow(clippy::if_same_then_else)]
    fn macro_connected_peers_boolean_test(boolean: bool) {
        // Increment by 1.
        connected_peers_inc!(boolean);
        assert_eq!(if boolean { 1 } else { 0 }, Metrics::get_connected_peers());

        // Increment by 1.
        connected_peers_inc!(boolean);
        assert_eq!(if boolean { 2 } else { 0 }, Metrics::get_connected_peers());

        // Decrement by 1.
        connected_peers_dec!(boolean);
        assert_eq!(if boolean { 1 } else { 0 }, Metrics::get_connected_peers());

        // Increment by 1.
        connected_peers_inc!(boolean);
        assert_eq!(if boolean { 2 } else { 0 }, Metrics::get_connected_peers());

        // Decrement by 2.
        connected_peers_dec!(boolean);
        connected_peers_dec!(boolean);
        assert_eq!(if boolean { 0 } else { 0 }, Metrics::get_connected_peers());

        // Decrement by 1.
        connected_peers_dec!(boolean);
        assert_eq!(if boolean { -1 } else { 0 }, Metrics::get_connected_peers());

        // Increment by 1.
        connected_peers_inc!(boolean);
        assert_eq!(0, Metrics::get_connected_peers());
    }

    #[test_with_metrics]
    fn test_macro_connected_peers() {
        // Increment by 1.
        connected_peers_inc!();
        assert_eq!(1, Metrics::get_connected_peers());

        // Increment by 1.
        connected_peers_inc!();
        assert_eq!(2, Metrics::get_connected_peers());

        // Decrement by 1.
        connected_peers_dec!();
        assert_eq!(1, Metrics::get_connected_peers());

        // Increment by 1.
        connected_peers_inc!();
        assert_eq!(2, Metrics::get_connected_peers());

        // Decrement by 2.
        connected_peers_dec!();
        connected_peers_dec!();
        assert_eq!(0, Metrics::get_connected_peers());

        // Decrement by 1.
        connected_peers_dec!();
        assert_eq!(-1, Metrics::get_connected_peers());

        // Increment by 1.
        connected_peers_inc!();
        assert_eq!(0, Metrics::get_connected_peers());
    }

    #[test_with_metrics]
    fn test_macro_connected_peers_boolean() {
        macro_connected_peers_boolean_test(true);
        macro_connected_peers_boolean_test(false);
    }
}
