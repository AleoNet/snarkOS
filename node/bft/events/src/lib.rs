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

#![forbid(unsafe_code)]

mod batch_certified;
pub use batch_certified::BatchCertified;

mod batch_propose;
pub use batch_propose::BatchPropose;

mod batch_signature;
pub use batch_signature::BatchSignature;

mod block_request;
pub use block_request::BlockRequest;

mod block_response;
pub use block_response::{BlockResponse, DataBlocks};

mod certificate_request;
pub use certificate_request::CertificateRequest;

mod certificate_response;
pub use certificate_response::CertificateResponse;

mod challenge_request;
pub use challenge_request::ChallengeRequest;

mod challenge_response;
pub use challenge_response::ChallengeResponse;

mod disconnect;
pub use disconnect::{Disconnect, DisconnectReason};

mod helpers;
pub use helpers::*;

mod primary_ping;
pub use primary_ping::PrimaryPing;

mod transmission_request;
pub use transmission_request::TransmissionRequest;

mod transmission_response;
pub use transmission_response::TransmissionResponse;

mod validators_request;
pub use validators_request::ValidatorsRequest;

mod validators_response;
pub use validators_response::ValidatorsResponse;

mod worker_ping;
pub use worker_ping::WorkerPing;

use snarkos_node_sync_locators::BlockLocators;
use snarkvm::{
    console::prelude::{error, FromBytes, Network, Read, ToBytes, Write},
    ledger::{
        block::Block,
        narwhal::{BatchCertificate, BatchHeader, Data, Transmission, TransmissionID},
    },
    prelude::{Address, Field, Signature},
};

use anyhow::{bail, ensure, Result};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
pub use std::io::{self, Result as IoResult};
use std::{borrow::Cow, net::SocketAddr};

pub trait EventTrait: ToBytes + FromBytes {
    /// Returns the event name.
    fn name(&self) -> Cow<'static, str>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
// TODO (howardwu): For mainnet - Remove this clippy lint. The CertificateResponse should not
//  be a large enum variant, after removing the versioning.
#[allow(clippy::large_enum_variant)]
pub enum Event<N: Network> {
    BatchPropose(BatchPropose<N>),
    BatchSignature(BatchSignature<N>),
    BatchCertified(BatchCertified<N>),
    BlockRequest(BlockRequest),
    BlockResponse(BlockResponse<N>),
    CertificateRequest(CertificateRequest<N>),
    CertificateResponse(CertificateResponse<N>),
    ChallengeRequest(ChallengeRequest<N>),
    ChallengeResponse(ChallengeResponse<N>),
    Disconnect(Disconnect),
    PrimaryPing(PrimaryPing<N>),
    TransmissionRequest(TransmissionRequest<N>),
    TransmissionResponse(TransmissionResponse<N>),
    ValidatorsRequest(ValidatorsRequest),
    ValidatorsResponse(ValidatorsResponse<N>),
    WorkerPing(WorkerPing<N>),
}

impl<N: Network> From<DisconnectReason> for Event<N> {
    fn from(reason: DisconnectReason) -> Self {
        Self::Disconnect(Disconnect { reason })
    }
}

impl<N: Network> Event<N> {
    /// The version of the event protocol; it can be incremented in order to force users to update.
    pub const VERSION: u32 = 8;

    /// Returns the event name.
    #[inline]
    pub fn name(&self) -> Cow<'static, str> {
        match self {
            Self::BatchPropose(event) => event.name(),
            Self::BatchSignature(event) => event.name(),
            Self::BatchCertified(event) => event.name(),
            Self::BlockRequest(event) => event.name(),
            Self::BlockResponse(event) => event.name(),
            Self::CertificateRequest(event) => event.name(),
            Self::CertificateResponse(event) => event.name(),
            Self::ChallengeRequest(event) => event.name(),
            Self::ChallengeResponse(event) => event.name(),
            Self::Disconnect(event) => event.name(),
            Self::PrimaryPing(event) => event.name(),
            Self::TransmissionRequest(event) => event.name(),
            Self::TransmissionResponse(event) => event.name(),
            Self::ValidatorsRequest(event) => event.name(),
            Self::ValidatorsResponse(event) => event.name(),
            Self::WorkerPing(event) => event.name(),
        }
    }

    /// Returns the event ID.
    #[inline]
    pub fn id(&self) -> u16 {
        match self {
            Self::BatchPropose(..) => 0,
            Self::BatchSignature(..) => 1,
            Self::BatchCertified(..) => 2,
            Self::BlockRequest(..) => 3,
            Self::BlockResponse(..) => 4,
            Self::CertificateRequest(..) => 5,
            Self::CertificateResponse(..) => 6,
            Self::ChallengeRequest(..) => 7,
            Self::ChallengeResponse(..) => 8,
            Self::Disconnect(..) => 9,
            Self::PrimaryPing(..) => 10,
            Self::TransmissionRequest(..) => 11,
            Self::TransmissionResponse(..) => 12,
            Self::ValidatorsRequest(..) => 13,
            Self::ValidatorsResponse(..) => 14,
            Self::WorkerPing(..) => 15,
        }
    }
}

impl<N: Network> ToBytes for Event<N> {
    fn write_le<W: io::Write>(&self, mut writer: W) -> IoResult<()> {
        self.id().write_le(&mut writer)?;

        match self {
            Self::BatchPropose(event) => event.write_le(writer),
            Self::BatchSignature(event) => event.write_le(writer),
            Self::BatchCertified(event) => event.write_le(writer),
            Self::BlockRequest(event) => event.write_le(writer),
            Self::BlockResponse(event) => event.write_le(writer),
            Self::CertificateRequest(event) => event.write_le(writer),
            Self::CertificateResponse(event) => event.write_le(writer),
            Self::ChallengeRequest(event) => event.write_le(writer),
            Self::ChallengeResponse(event) => event.write_le(writer),
            Self::Disconnect(event) => event.write_le(writer),
            Self::PrimaryPing(event) => event.write_le(writer),
            Self::TransmissionRequest(event) => event.write_le(writer),
            Self::TransmissionResponse(event) => event.write_le(writer),
            Self::ValidatorsRequest(event) => event.write_le(writer),
            Self::ValidatorsResponse(event) => event.write_le(writer),
            Self::WorkerPing(event) => event.write_le(writer),
        }
    }
}

impl<N: Network> FromBytes for Event<N> {
    fn read_le<R: io::Read>(mut reader: R) -> io::Result<Self> {
        // Read the event ID.
        let id = u16::read_le(&mut reader).map_err(|_| error("Unknown event ID"))?;

        // Deserialize the data field.
        let event = match id {
            0 => Self::BatchPropose(BatchPropose::read_le(&mut reader)?),
            1 => Self::BatchSignature(BatchSignature::read_le(&mut reader)?),
            2 => Self::BatchCertified(BatchCertified::read_le(&mut reader)?),
            3 => Self::BlockRequest(BlockRequest::read_le(&mut reader)?),
            4 => Self::BlockResponse(BlockResponse::read_le(&mut reader)?),
            5 => Self::CertificateRequest(CertificateRequest::read_le(&mut reader)?),
            6 => Self::CertificateResponse(CertificateResponse::read_le(&mut reader)?),
            7 => Self::ChallengeRequest(ChallengeRequest::read_le(&mut reader)?),
            8 => Self::ChallengeResponse(ChallengeResponse::read_le(&mut reader)?),
            9 => Self::Disconnect(Disconnect::read_le(&mut reader)?),
            10 => Self::PrimaryPing(PrimaryPing::read_le(&mut reader)?),
            11 => Self::TransmissionRequest(TransmissionRequest::read_le(&mut reader)?),
            12 => Self::TransmissionResponse(TransmissionResponse::read_le(&mut reader)?),
            13 => Self::ValidatorsRequest(ValidatorsRequest::read_le(&mut reader)?),
            14 => Self::ValidatorsResponse(ValidatorsResponse::read_le(&mut reader)?),
            15 => Self::WorkerPing(WorkerPing::read_le(&mut reader)?),
            16.. => return Err(error("Unknown event ID {id}")),
        };

        // Ensure that there are no "dangling" bytes.
        if reader.bytes().next().is_some() {
            return Err(error("Leftover bytes in an Event"));
        }

        Ok(event)
    }
}

#[cfg(test)]
mod tests {
    use crate::Event;
    use bytes::{Buf, BufMut, BytesMut};
    use snarkvm::console::prelude::{FromBytes, ToBytes};
    type CurrentNetwork = snarkvm::prelude::MainnetV0;

    #[test]
    fn deserializing_invalid_data_panics() {
        let buf = BytesMut::default();
        let invalid_id = u16::MAX;
        invalid_id.write_le(&mut buf.clone().writer()).unwrap();
        assert_eq!(
            Event::<CurrentNetwork>::read_le(buf.reader()).unwrap_err().to_string(),
            format!("Unknown event ID")
        );
    }
}

#[cfg(test)]
pub mod prop_tests {
    use crate::{
        batch_certified::prop_tests::any_batch_certified,
        batch_propose::prop_tests::any_batch_propose,
        batch_signature::prop_tests::any_batch_signature,
        certificate_request::prop_tests::any_certificate_request,
        certificate_response::prop_tests::any_certificate_response,
        challenge_request::prop_tests::any_challenge_request,
        challenge_response::prop_tests::any_challenge_response,
        transmission_request::prop_tests::any_transmission_request,
        transmission_response::prop_tests::any_transmission_response,
        worker_ping::prop_tests::any_worker_ping,
        Disconnect,
        DisconnectReason,
        Event,
    };
    use snarkvm::{
        console::{network::Network, types::Field},
        ledger::{narwhal::TransmissionID, puzzle::SolutionID},
        prelude::{FromBytes, Rng, ToBytes, Uniform},
    };

    use proptest::{
        prelude::{any, BoxedStrategy, Just, Strategy},
        prop_oneof,
        sample::Selector,
    };
    use test_strategy::proptest;

    type CurrentNetwork = snarkvm::prelude::MainnetV0;

    /// Returns the current UTC epoch timestamp.
    pub fn now() -> i64 {
        time::OffsetDateTime::now_utc().unix_timestamp()
    }

    pub fn any_solution_id() -> BoxedStrategy<SolutionID<CurrentNetwork>> {
        Just(0).prop_perturb(|_, mut rng| rng.gen::<u64>().into()).boxed()
    }

    pub fn any_transaction_id() -> BoxedStrategy<<CurrentNetwork as Network>::TransactionID> {
        Just(0)
            .prop_perturb(|_, mut rng| <CurrentNetwork as Network>::TransactionID::from(Field::rand(&mut rng)))
            .boxed()
    }

    pub fn any_transmission_checksum() -> BoxedStrategy<<CurrentNetwork as Network>::TransmissionChecksum> {
        Just(0).prop_perturb(|_, mut rng| rng.gen::<<CurrentNetwork as Network>::TransmissionChecksum>()).boxed()
    }

    pub fn any_transmission_id() -> BoxedStrategy<TransmissionID<CurrentNetwork>> {
        prop_oneof![
            (any_transaction_id(), any_transmission_checksum())
                .prop_map(|(id, cs)| TransmissionID::Transaction(id, cs)),
            (any_solution_id(), any_transmission_checksum()).prop_map(|(id, cs)| TransmissionID::Solution(id, cs)),
        ]
        .boxed()
    }

    pub fn any_event() -> BoxedStrategy<Event<CurrentNetwork>> {
        prop_oneof![
            any_batch_certified().prop_map(Event::BatchCertified),
            any_batch_propose().prop_map(Event::BatchPropose),
            any_batch_signature().prop_map(Event::BatchSignature),
            any_certificate_request().prop_map(Event::CertificateRequest),
            any_certificate_response().prop_map(Event::CertificateResponse),
            any_challenge_request().prop_map(Event::ChallengeRequest),
            any_challenge_response().prop_map(Event::ChallengeResponse),
            (
                Just(vec![
                    DisconnectReason::ProtocolViolation,
                    DisconnectReason::NoReasonGiven,
                    DisconnectReason::InvalidChallengeResponse,
                    DisconnectReason::OutdatedClientVersion,
                ]),
                any::<Selector>()
            )
                .prop_map(|(reasons, selector)| Event::Disconnect(Disconnect::from(selector.select(reasons)))),
            any_transmission_request().prop_map(Event::TransmissionRequest),
            any_transmission_response().prop_map(Event::TransmissionResponse),
            any_worker_ping().prop_map(Event::WorkerPing)
        ]
        .boxed()
    }

    #[proptest]
    fn serialize_deserialize(#[strategy(any_event())] original: Event<CurrentNetwork>) {
        let mut buf = Vec::new();
        Event::write_le(&original, &mut buf).unwrap();

        let deserialized: Event<CurrentNetwork> = Event::read_le(&*buf).unwrap();
        assert_eq!(original.id(), deserialized.id());
        assert_eq!(original.name(), deserialized.name());
    }
}
