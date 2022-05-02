#![no_main]
use libfuzzer_sys::fuzz_target;

use snarkos_environment::{network::Message, Client, CurrentNetwork};

// This fuzz target tests network message deserialization, including
// the deferred deserialization of the heavier objects.

// To start fuzzing, run `cargo +nightly fuzz run deserialization`.

fuzz_target!(|message: &[u8]| {
    if let Ok(message) = Message::<CurrentNetwork, Client<CurrentNetwork>>::deserialize(message.into()) {
        match message {
            Message::BlockResponse(data) => {
                let _ = data.deserialize_blocking();
            }
            Message::ChallengeResponse(data) => {
                let _ = data.deserialize_blocking();
            }
            Message::Ping(.., data) => {
                let _ = data.deserialize_blocking();
            }
            Message::Pong(.., data) => {
                let _ = data.deserialize_blocking();
            }
            Message::UnconfirmedBlock(.., data) => {
                let _ = data.deserialize_blocking();
            }
            Message::UnconfirmedTransaction(data) => {
                let _ = data.deserialize_blocking();
            }
            Message::PoolRequest(.., data) => {
                let _ = data.deserialize_blocking();
            }
            Message::PoolResponse(.., data) => {
                let _ = data.deserialize_blocking();
            }
            _ => {}
        }
    }
});
