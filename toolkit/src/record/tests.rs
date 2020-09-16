// Copyright (C) 2019-2020 Aleo Systems Inc.
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

use crate::{
    account::{PrivateKey, ViewKey},
    record::Record,
};

use std::str::FromStr;

// The test data for records in `snarkos-toolkit`
const ENCRYPTED_RECORD_STRING: &str = "0841ad8eb642c4a2475e2d6c3548f445253db290842531d9b5e25effe74d3eee03c097f5273f56517fe1615100f820577619242101568ddc5da5972b7b7c1c760a6969ddc7ed39cd774a18bc15d5cf38c6d59df1d14e05add65f0e4e6a54b2c901f1580a556f9e9f8e438cdb0d92fa0da1642816eb9318c14387be499d7481950847131dbb8496d3dcc58811dfa96df2bd2ad769cb69438bb1a2657625686b140f1196bfe7a292673f8502acc9cd1ac30f0d16342759105882b3026dafa030320285daefd9fde6dc65dd33541452b43a3bf17e57cf2f147392edc8f8c65af3850020b79c96609743cbfd0b21249265c84344e1c993b480cd042e296d66c17bc7056500";
const RECORD_STRING: &str = "4f6d042c3bc73e412f4b4740ad27354a1b25bb9df93f29313350356aa88dca050064000000000000000000000000000000000000000000000000000000000000000000000000000000304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00d3cb77fba8ef681178b0dd54dedae5d8c72ea496acb71a788a9bf3be8cf552082ffd75e3e518853ef2328b72b29cda505241bf26b264e85cb12556e3d4556b03e095bb02db522a7dd926a2dff52838bb541dd9cf6eb07c416deeee6f30e54a02";
const SERIAL_NUMBER_STRING: &str = "7b5ead0c963658ed7127bcdcb916eb42397824ee82a23c8c3435a60469630410afc2550c22cb9c3b26c5899a4b8150f12b75cfcc032d21dd735b1feb5d87e50e";
const PRIVATE_KEY_STRING: &str = "APrivateKey1tvv5YV1dipNiku2My8jMkqpqCyYKvR5Jq4y2mtjw7s77Zpn";

#[test]
pub fn record_test() {
    let record = Record::from_str(RECORD_STRING);
    assert!(record.is_ok());

    let candidate_record = record.unwrap().to_string();

    println!("{} == {}", RECORD_STRING, candidate_record);
    assert_eq!(RECORD_STRING, candidate_record);
}

#[test]
pub fn serial_number_derivation_test() {
    let record = Record::from_str(RECORD_STRING).unwrap();
    let private_key = PrivateKey::from_str(PRIVATE_KEY_STRING).unwrap();

    let serial_number = record.derive_serial_number(&private_key);
    assert!(serial_number.is_ok());

    let candidate_serial_number = hex::encode(serial_number.unwrap());

    println!("{} == {}", SERIAL_NUMBER_STRING, candidate_serial_number);
    assert_eq!(SERIAL_NUMBER_STRING, candidate_serial_number);
}

#[test]
pub fn decrypt_record_test() {
    let private_key = PrivateKey::from_str(PRIVATE_KEY_STRING).unwrap();
    let view_key = ViewKey::from(&private_key).unwrap();

    let record = Record::decrypt_record(ENCRYPTED_RECORD_STRING, &view_key);
    assert!(record.is_ok());

    let candidate_record = record.unwrap().to_string();

    println!("{} == {}", RECORD_STRING, candidate_record);
    assert_eq!(RECORD_STRING, candidate_record);
}
