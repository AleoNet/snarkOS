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

mod batch_certified;
pub use batch_certified::BatchCertified;

mod batch_propose;
pub use batch_propose::BatchPropose;

mod batch_signature;
pub use batch_signature::BatchSignature;

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

mod primary_ping;
pub use primary_ping::PrimaryPing;

mod transmission_request;
pub use transmission_request::TransmissionRequest;

mod transmission_response;
pub use transmission_response::TransmissionResponse;

mod worker_ping;
pub use worker_ping::WorkerPing;

use snarkos_node_sync::locators::BlockLocators;
use snarkvm::{
    console::prelude::{FromBytes, Network, Read, ToBytes, Write},
    ledger::narwhal::{BatchCertificate, BatchHeader, Data, Transmission, TransmissionID},
    prelude::{Address, Field, Signature},
};

use ::bytes::{Buf, BytesMut};
use anyhow::{bail, Result};
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
pub use std::io::Result as IoResult;

pub trait EventTrait: ToBytes + FromBytes {
    /// Returns the event name.
    fn name(&self) -> Cow<'static, str>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event<N: Network> {
    BatchPropose(BatchPropose<N>),
    BatchSignature(BatchSignature<N>),
    BatchCertified(BatchCertified<N>),
    CertificateRequest(CertificateRequest<N>),
    CertificateResponse(CertificateResponse<N>),
    ChallengeRequest(ChallengeRequest<N>),
    ChallengeResponse(ChallengeResponse<N>),
    Disconnect(Disconnect),
    PrimaryPing(PrimaryPing<N>),
    TransmissionRequest(TransmissionRequest<N>),
    TransmissionResponse(TransmissionResponse<N>),
    WorkerPing(WorkerPing<N>),
}

impl<N: Network> From<DisconnectReason> for Event<N> {
    fn from(reason: DisconnectReason) -> Self {
        Self::Disconnect(Disconnect { reason })
    }
}

impl<N: Network> Event<N> {
    /// The version of the event protocol; it can be incremented in order to force users to update.
    pub const VERSION: u32 = 1;

    /// Returns the event name.
    #[inline]
    pub fn name(&self) -> Cow<'static, str> {
        match self {
            Self::BatchPropose(event) => event.name(),
            Self::BatchSignature(event) => event.name(),
            Self::BatchCertified(event) => event.name(),
            Self::CertificateRequest(event) => event.name(),
            Self::CertificateResponse(event) => event.name(),
            Self::ChallengeRequest(event) => event.name(),
            Self::ChallengeResponse(event) => event.name(),
            Self::Disconnect(event) => event.name(),
            Self::PrimaryPing(event) => event.name(),
            Self::TransmissionRequest(event) => event.name(),
            Self::TransmissionResponse(event) => event.name(),
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
            Self::CertificateRequest(..) => 3,
            Self::CertificateResponse(..) => 4,
            Self::ChallengeRequest(..) => 5,
            Self::ChallengeResponse(..) => 6,
            Self::Disconnect(..) => 7,
            Self::PrimaryPing(..) => 8,
            Self::TransmissionRequest(..) => 9,
            Self::TransmissionResponse(..) => 10,
            Self::WorkerPing(..) => 11,
        }
    }

    /// Serializes the event into the buffer.
    #[inline]
    pub fn serialize<W: Write>(&self, writer: &mut W) -> IoResult<()> {
        self.id().write_le(&mut *writer)?;

        match self {
            Self::BatchPropose(event) => event.write_le(writer),
            Self::BatchSignature(event) => event.write_le(writer),
            Self::BatchCertified(event) => event.write_le(writer),
            Self::CertificateRequest(event) => event.write_le(writer),
            Self::CertificateResponse(event) => event.write_le(writer),
            Self::ChallengeRequest(event) => event.write_le(writer),
            Self::ChallengeResponse(event) => event.write_le(writer),
            Self::Disconnect(event) => event.write_le(writer),
            Self::PrimaryPing(event) => event.write_le(writer),
            Self::TransmissionRequest(event) => event.write_le(writer),
            Self::TransmissionResponse(event) => event.write_le(writer),
            Self::WorkerPing(event) => event.write_le(writer),
        }
    }

    /// Deserializes the given buffer into a event.
    #[inline]
    pub fn deserialize(mut bytes: BytesMut) -> Result<Self> {
        // Ensure there is at least a event ID in the buffer.
        if bytes.remaining() < 2 {
            bail!("Missing event ID");
        }

        // Read the event ID.
        let id: u16 = bytes.get_u16_le();

        // Prepare a reader.
        let reader = bytes.reader();

        // Deserialize the data field.
        let event = match id {
            0 => Self::BatchPropose(BatchPropose::read_le(reader)?),
            1 => Self::BatchSignature(BatchSignature::read_le(reader)?),
            2 => Self::BatchCertified(BatchCertified::read_le(reader)?),
            3 => Self::CertificateRequest(CertificateRequest::read_le(reader)?),
            4 => Self::CertificateResponse(CertificateResponse::read_le(reader)?),
            5 => Self::ChallengeRequest(ChallengeRequest::read_le(reader)?),
            6 => Self::ChallengeResponse(ChallengeResponse::read_le(reader)?),
            7 => Self::Disconnect(Disconnect::read_le(reader)?),
            8 => Self::PrimaryPing(PrimaryPing::read_le(reader)?),
            9 => Self::TransmissionRequest(TransmissionRequest::read_le(reader)?),
            10 => Self::TransmissionResponse(TransmissionResponse::read_le(reader)?),
            11 => Self::WorkerPing(WorkerPing::read_le(reader)?),
            _ => bail!("Unknown event ID {id}"),
        };

        Ok(event)
    }
}

#[cfg(test)]
pub mod prop_tests {
    use crate::{
        event::{
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
        },
        Disconnect,
        DisconnectReason,
        Event,
    };
    use bytes::{BufMut, BytesMut};
    use proptest::{
        prelude::{any, BoxedStrategy, Just, Strategy},
        prop_oneof,
        sample::Selector,
    };
    use test_strategy::proptest;

    type CurrentNetwork = snarkvm::prelude::Testnet3;

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
        let mut buf = BytesMut::default().writer();
        Event::serialize(&original, &mut buf).unwrap();

        let deserialized: Event<CurrentNetwork> = Event::deserialize(buf.get_ref().clone()).unwrap();
        assert_eq!(original.id(), deserialized.id());
        assert_eq!(original.name(), deserialized.name());
    }
}

#[cfg(test)]
mod tests {
    use crate::Event;
    use bytes::{BufMut, BytesMut};
    use snarkvm::console::prelude::ToBytes;
    type CurrentNetwork = snarkvm::prelude::Testnet3;

    #[test]
    #[should_panic(expected = "Missing event ID")]
    fn deserializing_empty_defaults_no_reason() {
        let buf = BytesMut::default().writer();
        Event::<CurrentNetwork>::deserialize(buf.get_ref().clone()).unwrap();
    }

    #[test]
    fn deserializing_invalid_data_panics() {
        let mut buf = BytesMut::default().writer();
        let invalid_id = u16::MAX;
        invalid_id.write_le(&mut buf).unwrap();
        assert_eq!(
            Event::<CurrentNetwork>::deserialize(buf.get_ref().clone()).unwrap_err().to_string(),
            format!("Unknown event ID {invalid_id}")
        );
    }
}
