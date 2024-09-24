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

pub const BECH32M_CHARSET: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l1";

/// Check if a string is a valid bech32m character set.
///
/// A bech32m character set is considered valid if it consists of the following characters:
/// ```ignore
///     qpzry9x8gf2tvdw0s3jn54khce6mua7l1
/// ```
/// The function returns `true` if the string is a valid bech32m character set, and `false` otherwise.
pub fn is_in_bech32m_charset(s: &str) -> bool {
    s.as_bytes().iter().all(|b| BECH32M_CHARSET.as_bytes().contains(b))
}

/// Check if a given vanity string exists at the start or end of the data part of a bech32m string.
///
/// The bech32m string must have the following format:
/// ```ignore
///     <HRP>1<data>[<vanity string>]
/// ```
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
    let (hrp, data) = match s.split_once('1') {
        Some((hrp, data)) => (hrp, data),
        // The bech32m string is invalid.
        None => return false,
    };
    // Ensure neither the HRP nor the data part are empty.
    if hrp.is_empty() || data.is_empty() {
        return false;
    }
    // Check if the vanity string exists at the start or end of the data part.
    data.starts_with(vanity) || data.ends_with(vanity)
}

#[test]
fn test_is_in_bech32m_charset() {
    assert!(is_in_bech32m_charset("qpzry9x8gf2tvdw0s3jn54khce6mua7l1qpzry9x8gf2tvdw0s3jn54khce6mua7l1"));
    assert!(!is_in_bech32m_charset("qpzry9x8gf2tvdw0s3jn54khce6mua7l1qpzry9x8gf2tvdw0s3jn54khce6mua7lo"));
}

#[test]
fn test_has_vanity_string() {
    assert!(has_vanity_string("myhrp1myvanitystring", "myvanitystring"));
    assert!(!has_vanity_string("myhrp1myvanitystring", "anotherstring"));
    assert!(has_vanity_string("myhrp1myvanitystring1234", "myvanitystring"));
    assert!(has_vanity_string("myhrp11234myvanitystring", "myvanitystring"));
    assert!(!has_vanity_string("myhrp1anotherstring1234", "myvanitystring"));
}
