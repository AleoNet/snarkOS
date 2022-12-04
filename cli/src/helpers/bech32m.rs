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

/// Check if a string is a valid bech32m character set.
///
/// A bech32m character set is considered valid if it consists of the following characters:
///
///     qpzry9x8gf2tvdw0s3jn54khce6mua7l1
///
/// The function returns `true` if the string is a valid bech32m character set, and `false` otherwise.
pub fn is_valid_bech32m_charset(s: &str) -> bool {
    s.as_bytes().iter().all(|b| BECH32M_CHARSET.contains(b))
}

/// Check if a given vanity string exists at the start or end of the data part of a bech32m string.
///
/// The bech32m string must have the following format:
///
///     <HRP>1<data>[<vanity string>]
///
/// where:
///
/// - `<HRP>` is the human-readable part of the bech32m string.
/// - `1` is the separator between the HRP and the data part.
/// - `<data>` is the data part of the bech32m string.
/// - `<vanity string>` is the vanity string to search for. This string may or may not be present at
///   the start or end of the data part.
///
/// The function returns `true` if the vanity string exists at the start or end of the data part, and
/// `false` otherwise.
pub fn has_vanity_string(s: &str, vanity: &str) -> bool {
    // Split the bech32m string into the HRP and data parts.
    let parts: Vec<&str> = s.split("1").collect();
    if parts.len() != 2 {
        return false;
    }
    let (_hrp, data) = (parts[0], parts[1]);

    // Check if the vanity string exists at the start or end of the data part.
    data.starts_with(vanity) || data.ends_with(vanity)
}

#[test]
fn test_is_valid_bech32m_charset() {
    assert!(is_valid_bech32m_charset("qpzry9x8gf2tvdw0s3jn54khce6mua7l1qpzry9x8gf2tvdw0s3jn54khce6mua7l1"));
    assert!(!is_valid_bech32m_charset("qpzry9x8gf2tvdw0s3jn54khce6mua7l1qpzry9x8gf2tvdw0s3jn54khce6mua7l2"));
}

#[test]
fn test_has_vanity_string() {
    assert!(has_vanity_string("myhrp1myvanitystring", "myvanitystring"));
    assert!(!has_vanity_string("myhrp1myvanitystring", "anotherstring"));
    assert!(has_vanity_string("myhrp1myvanitystring1234", "myvanitystring"));
    assert!(has_vanity_string("myhrp11234myvanitystring", "myvanitystring"));
    assert!(!has_vanity_string("myhrp1anotherstring1234", "myvanitystring"));
}
