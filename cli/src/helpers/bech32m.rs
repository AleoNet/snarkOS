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

const BECH32M_CHARSET: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l1";

/// Returns `true` if the given string is a valid bech32m string.
pub fn is_valid_bech32m_string(s: &str) -> bool {
    s.as_bytes().iter().all(|b| BECH32M_CHARSET.contains(b))
}

#[test]
fn test_is_valid_bech32m_string() {
    assert!(is_valid_bech32m_string("qpzry9x8gf2tvdw0s3jn54khce6mua7l1qpzry9x8gf2tvdw0s3jn54khce6mua7l1"));
    assert!(!is_valid_bech32m_string("qpzry9x8gf2tvdw0s3jn54khce6mua7l1qpzry9x8gf2tvdw0s3jn54khce6mua7l2"));
}
