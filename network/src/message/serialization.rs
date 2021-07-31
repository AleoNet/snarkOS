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

use snarkvm_ledger::BlockHeaderHash;

use crate::message::{Payload, Version};
use payload_capnp::{
    block,
    block_hash,
    payload::{
        payload_type,
        {self},
    },
    socket_addr,
    transaction,
    version,
};

use std::{
    io,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
};

pub mod payload_capnp {
    include!("payload_capnp.rs");
}

type BlockHashes<'a> = capnp::struct_list::Reader<'a, block_hash::Owned>;
type SocketAddrs<'a> = capnp::struct_list::Reader<'a, socket_addr::Owned>;
type Transactions<'a> = capnp::struct_list::Reader<'a, transaction::Owned>;

impl Version {
    pub fn deserialize(bytes: &[u8]) -> capnp::Result<Version> {
        let mut cursor = io::Cursor::new(bytes);
        let message_reader = capnp::serialize_packed::read_message(&mut cursor, capnp::message::ReaderOptions::new())?;

        let version = message_reader.get_root::<version::Reader>()?;

        Ok(Version {
            version: version.get_version(),
            listening_port: version.get_listening_port(),
            node_id: version.get_node_id(),
        })
    }

    pub fn serialize(&self) -> capnp::Result<Vec<u8>> {
        let mut message = capnp::message::Builder::new_default();
        let mut builder = message.init_root::<version::Builder>();
        builder.set_version(self.version);
        builder.set_listening_port(self.listening_port);
        builder.set_node_id(self.node_id);

        let mut writer = Vec::new();
        capnp::serialize_packed::write_message(&mut writer, &message)?;
        Ok(writer)
    }
}

impl Payload {
    pub fn deserialize(bytes: &[u8]) -> capnp::Result<Payload> {
        let mut cursor = io::Cursor::new(bytes);
        let message_reader = capnp::serialize_packed::read_message(&mut cursor, capnp::message::ReaderOptions::new())?;

        let payload = message_reader.get_root::<payload::Reader>()?.get_payload_type();

        let payload_type = match payload.which() {
            Ok(pt) => pt,
            Err(capnp::NotInSchema(_)) => return Ok(Payload::Unknown), // future-proofing
        };

        match payload_type {
            payload_type::Which::Block(block) => deserialize_block(block?, false),
            payload_type::Which::GetBlocks(hashes) => Ok(Payload::GetBlocks(deserialize_block_hashes(hashes?)?)),
            payload_type::Which::GetMemoryPool(_) => Ok(Payload::GetMemoryPool),
            payload_type::Which::GetPeers(_) => Ok(Payload::GetPeers),
            payload_type::Which::GetSync(hashes) => Ok(Payload::GetSync(deserialize_block_hashes(hashes?)?)),
            payload_type::Which::MemoryPool(txs) => deserialize_transactions(txs?),
            payload_type::Which::Peers(peers) => Ok(Payload::Peers(deserialize_addresses(peers?)?)),
            payload_type::Which::Ping(ping) => Ok(Payload::Ping(ping?.get_block_height())),
            payload_type::Which::Pong(_) => Ok(Payload::Pong),
            payload_type::Which::Sync(hashes) => Ok(Payload::Sync(deserialize_block_hashes(hashes?)?)),
            payload_type::Which::SyncBlock(block) => deserialize_block(block?, true),
            payload_type::Which::Transaction(tx) => Ok(Payload::Transaction(tx?.get_data()?.to_vec())),
        }
    }

    pub fn serialize(&self) -> capnp::Result<Vec<u8>> {
        let mut message = capnp::message::Builder::new_default();

        {
            let builder = message.init_root::<payload::Builder>().init_payload_type();

            match self {
                Payload::Block(bytes) => {
                    let mut builder = builder.init_block();
                    builder.set_data(bytes);
                }
                Payload::GetBlocks(hashes) => {
                    let mut builder = builder.init_get_blocks(hashes.len() as u32);
                    for (i, hash) in hashes.iter().enumerate() {
                        let mut elem_builder = builder.reborrow().get(i as u32);
                        elem_builder.set_hash(&hash.0);
                    }
                }
                Payload::GetMemoryPool => {
                    let mut builder = builder.init_get_memory_pool();
                    builder.set_placeholder(());
                }
                Payload::GetPeers => {
                    let mut builder = builder.init_get_peers();
                    builder.set_placeholder(());
                }
                Payload::GetSync(hashes) => {
                    let mut builder = builder.init_get_sync(hashes.len() as u32);
                    for (i, hash) in hashes.iter().enumerate() {
                        let mut elem_builder = builder.reborrow().get(i as u32);
                        elem_builder.set_hash(&hash.0);
                    }
                }
                Payload::MemoryPool(txs) => {
                    let mut builder = builder.init_memory_pool(txs.len() as u32);
                    for (i, tx) in txs.iter().enumerate() {
                        let mut elem_builder = builder.reborrow().get(i as u32);
                        elem_builder.set_data(tx);
                    }
                }
                Payload::Peers(addrs) => {
                    let mut builder = builder.init_peers(addrs.len() as u32);
                    for (i, addr) in addrs.iter().enumerate() {
                        let elem_builder = builder.reborrow().get(i as u32);
                        let elem_builder = elem_builder.init_addr_type();
                        match addr {
                            SocketAddr::V4(addr) => {
                                let mut addr_builder = elem_builder.init_v4();
                                addr_builder.set_port(addr.port());
                                let addr_builder = addr_builder.init_addr();
                                let mut addr_builder = addr_builder.init_octets(4);
                                for (i, octet) in addr.ip().octets().iter().enumerate() {
                                    addr_builder.set(i as u32, *octet);
                                }
                            }
                            SocketAddr::V6(addr) => {
                                let mut addr_builder = elem_builder.init_v6();
                                addr_builder.set_port(addr.port());
                                let addr_builder = addr_builder.init_addr();
                                let mut addr_builder = addr_builder.init_octets(16);
                                for (i, octet) in addr.ip().octets().iter().enumerate() {
                                    addr_builder.set(i as u32, *octet);
                                }
                            }
                        }
                    }
                }
                Payload::Ping(block_height) => {
                    let mut builder = builder.init_ping();
                    builder.set_block_height(*block_height);
                }
                Payload::Pong => {
                    let mut builder = builder.init_pong();
                    builder.set_placeholder(());
                }
                Payload::Sync(hashes) => {
                    let mut builder = builder.init_sync(hashes.len() as u32);
                    for (i, hash) in hashes.iter().enumerate() {
                        let mut elem_builder = builder.reborrow().get(i as u32);
                        elem_builder.set_hash(&hash.0);
                    }
                }
                Payload::SyncBlock(bytes) => {
                    let mut builder = builder.init_sync_block();
                    builder.set_data(bytes);
                }
                Payload::Transaction(bytes) => {
                    let mut builder = builder.init_transaction();
                    builder.set_data(bytes);
                }
                _ => unreachable!(),
            }
        }

        let mut writer = Vec::new();
        capnp::serialize_packed::write_message(&mut writer, &message)?;
        Ok(writer)
    }
}

fn deserialize_block(block: block::Reader<'_>, is_sync: bool) -> capnp::Result<Payload> {
    let data = block.get_data()?.to_vec();

    let payload = if is_sync {
        Payload::SyncBlock(data)
    } else {
        Payload::Block(data)
    };

    Ok(payload)
}

fn deserialize_block_hashes(hashes: BlockHashes<'_>) -> capnp::Result<Vec<BlockHeaderHash>> {
    let mut vec = Vec::with_capacity(hashes.len() as usize);

    for hash in hashes.iter() {
        let bytes = hash.get_hash()?;
        let mut block_hash = [0u8; 32];
        block_hash.copy_from_slice(bytes);
        vec.push(BlockHeaderHash(block_hash));
    }

    Ok(vec)
}

fn deserialize_addresses(addrs: SocketAddrs<'_>) -> capnp::Result<Vec<SocketAddr>> {
    let mut vec = Vec::with_capacity(addrs.len() as usize);

    for addr in addrs.iter() {
        let addr = addr.get_addr_type();
        let addr = match addr.which()? {
            // TODO(ljedrz/nkls): deduplicate the branches using a macro
            socket_addr::addr_type::V4(addr) => {
                let addr = addr?;
                let ip = addr.get_addr()?;
                let mut octets = [0u8; 4];
                for (i, octet) in ip.get_octets()?.iter().enumerate() {
                    if i > 3 {
                        return Err(capnp::Error {
                            kind: capnp::ErrorKind::Failed,
                            description: "invalid IPv4 address: too many octets".to_owned(),
                        });
                    }
                    octets[i] = octet;
                }
                let ip = Ipv4Addr::from(octets);
                let port = addr.get_port();

                SocketAddr::from((ip, port))
            }
            socket_addr::addr_type::V6(addr) => {
                let addr = addr?;
                let ip = addr.get_addr()?;
                let mut octets = [0u8; 16];
                for (i, octet) in ip.get_octets()?.iter().enumerate() {
                    if i > 15 {
                        return Err(capnp::Error {
                            kind: capnp::ErrorKind::Failed,
                            description: "invalid IPv6 address: too many octets".to_owned(),
                        });
                    }
                    octets[i] = octet;
                }
                let ip = Ipv6Addr::from(octets);
                let port = addr.get_port();

                SocketAddr::from((ip, port))
            }
        };
        vec.push(addr);
    }

    Ok(vec)
}

fn deserialize_transactions(txs: Transactions<'_>) -> capnp::Result<Payload> {
    let mut vec = Vec::with_capacity(txs.len() as usize);

    for tx in txs.iter() {
        let bytes = tx.get_data()?;
        vec.push(bytes.to_vec());
    }

    Ok(Payload::MemoryPool(vec))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_deserialize_empty_payloads() {
        for payload in &[Payload::GetMemoryPool, Payload::GetPeers, Payload::Pong] {
            assert_eq!(
                Payload::deserialize(&Payload::serialize(payload).unwrap()).unwrap(),
                *payload
            );
        }
    }

    #[test]
    fn serialize_deserialize_payloads_with_blobs() {
        let blob = (0u8..255).collect::<Vec<_>>();

        for payload in &[
            Payload::Block(blob.clone()),
            Payload::MemoryPool(vec![blob.clone(); 10]),
            Payload::SyncBlock(blob.clone()),
            Payload::Transaction(blob),
        ] {
            assert_eq!(
                Payload::deserialize(&Payload::serialize(payload).unwrap()).unwrap(),
                *payload
            );
        }
    }

    #[test]
    fn serialize_deserialize_payloads_with_hashes() {
        let hashes = (0u8..10).map(|i| BlockHeaderHash::new(vec![i; 32])).collect::<Vec<_>>();

        for payload in &[
            Payload::GetBlocks(hashes.clone()),
            Payload::GetSync(hashes.clone()),
            Payload::Sync(hashes),
        ] {
            assert_eq!(
                Payload::deserialize(&Payload::serialize(payload).unwrap()).unwrap(),
                *payload
            );
        }
    }

    #[test]
    fn serialize_deserialize_peers() {
        let addrs: Vec<SocketAddr> = [
            "0.0.0.0:0",
            "127.0.0.1:4141",
            "192.168.1.1:14131",
            "[::1]:0",
            "[2001:0db8:85a3:0000:0000:8a2e:0370:7334]:14131",
            "[::ffff:192.0.2.128]:4141",
        ]
        .iter()
        .map(|addr| addr.parse().unwrap())
        .collect();
        let payload = Payload::Peers(addrs);

        assert_eq!(
            Payload::deserialize(&Payload::serialize(&payload).unwrap()).unwrap(),
            payload
        );
    }

    #[test]
    fn serialize_deserialize_ping() {
        for i in 0u8..255 {
            let payload = Payload::Ping(i as u32);

            assert_eq!(
                Payload::deserialize(&Payload::serialize(&payload).unwrap()).unwrap(),
                payload
            );
        }
    }

    #[test]
    fn serialize_deserialize_version() {
        let version = Version::new(crate::PROTOCOL_VERSION, 4141, 0);

        assert_eq!(
            Version::deserialize(&Version::serialize(&version).unwrap()).unwrap(),
            version
        );
    }
}
