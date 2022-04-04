#![no_main]
use libfuzzer_sys::fuzz_target;

use bytes::BytesMut;
use snarkos_environment::{network::MessageCodec, Client, CurrentNetwork};
use tokio_util::codec::Decoder;

// This fuzz target tests network message buffering and decoding; it does NOT include
// the deferred deserialization of the heavier objects.

// To start fuzzing, run `cargo +nightly fuzz run network`.

fuzz_target!(|messages: Vec<&[u8]>| {
    let mut codec = MessageCodec::<CurrentNetwork, Client<CurrentNetwork>>::default();

    for message in messages {
        let mut bytes = BytesMut::from(message);
        let _ = codec.decode(&mut bytes);
    }
});
