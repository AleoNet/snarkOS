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

#![no_main]

use std::sync::Mutex;

use snarkos_node_bft_events::{
    Event, EventOrBytes, NoiseCodec, NoiseState, TransmissionResponse, NOISE_HANDSHAKE_TYPE,
};
use snarkvm::{
    ledger::narwhal::{Data, Transmission, TransmissionID},
    prelude::{Field, Network, TestRng, Uniform},
};

use bytes::{Bytes, BytesMut};
use libfuzzer_sys::fuzz_target;
use once_cell::sync::OnceCell;
use snow::{params::NoiseParams, Builder};
use tokio_util::codec::{Decoder, Encoder};

type CurrentNetwork = snarkvm::prelude::Testnet3;

static RNG: OnceCell<Mutex<TestRng>> = OnceCell::new();
static CODECS: OnceCell<Mutex<(NoiseCodec<CurrentNetwork>, NoiseCodec<CurrentNetwork>)>> = OnceCell::new();

fuzz_target!(|data: &[u8]| {
    let mut rng = &mut *RNG.get_or_init(|| Default::default()).lock().unwrap();

    let codecs = CODECS.get_or_init(|| handshake_xx());
    let (initiator_codec, responder_codec) = &mut *codecs.lock().unwrap();
    let mut ciphertext = BytesMut::new();

    let id = TransmissionID::Transaction(<CurrentNetwork as Network>::TransactionID::from(Field::rand(&mut rng)));
    let transmission = Transmission::Transaction(Data::Buffer(Bytes::copy_from_slice(data)));

    let transmission_response = TransmissionResponse::new(id, transmission);
    let msg = EventOrBytes::Event(Event::TransmissionResponse(transmission_response));

    assert!(initiator_codec.encode(msg.clone(), &mut ciphertext).is_ok());
    assert_eq!(responder_codec.decode(&mut ciphertext).unwrap().unwrap(), msg);
});

fn handshake_xx() -> Mutex<(NoiseCodec<CurrentNetwork>, NoiseCodec<CurrentNetwork>)> {
    let params: NoiseParams = NOISE_HANDSHAKE_TYPE.parse().unwrap();
    let initiator_builder = Builder::new(params.clone());
    let initiator_kp = initiator_builder.generate_keypair().unwrap();
    let initiator = initiator_builder.local_private_key(&initiator_kp.private).build_initiator().unwrap();

    let responder_builder = Builder::new(params);
    let responder_kp = responder_builder.generate_keypair().unwrap();
    let responder = responder_builder.local_private_key(&responder_kp.private).build_responder().unwrap();

    let mut initiator_codec = NoiseCodec::new(NoiseState::Handshake(Box::new(initiator)));
    let mut responder_codec = NoiseCodec::new(NoiseState::Handshake(Box::new(responder)));

    let mut ciphertext = BytesMut::new();

    // -> e
    assert!(initiator_codec.encode(EventOrBytes::Bytes(Bytes::new()), &mut ciphertext).is_ok());
    assert!(
        matches!(responder_codec.decode(&mut ciphertext).unwrap().unwrap(), EventOrBytes::Bytes(bytes) if bytes.is_empty())
    );

    // <- e, ee, s, es
    assert!(responder_codec.encode(EventOrBytes::Bytes(Bytes::new()), &mut ciphertext).is_ok());
    assert!(
        matches!(initiator_codec.decode(&mut ciphertext).unwrap().unwrap(), EventOrBytes::Bytes(bytes) if bytes.is_empty())
    );

    // -> s, se
    assert!(initiator_codec.encode(EventOrBytes::Bytes(Bytes::new()), &mut ciphertext).is_ok());
    assert!(
        matches!(responder_codec.decode(&mut ciphertext).unwrap().unwrap(), EventOrBytes::Bytes(bytes) if bytes.is_empty())
    );

    initiator_codec.noise_state = initiator_codec.noise_state.into_post_handshake_state();
    responder_codec.noise_state = responder_codec.noise_state.into_post_handshake_state();

    Mutex::new((initiator_codec, responder_codec))
}
