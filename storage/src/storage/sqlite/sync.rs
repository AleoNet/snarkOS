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

use std::{convert::TryInto, net::SocketAddr};

use chrono::{DateTime, NaiveDateTime, Utc};
use hash_hasher::HashedMap;
use rusqlite::{params, OptionalExtension, Row, ToSql};
use snarkvm_dpc::{AleoAmount, MerkleRootHash, Network, PedersenMerkleRootHash, ProofOfSuccinctWork};
use tracing::*;

#[cfg(feature = "test")]
use crate::key_value::KeyValueColumn;
use crate::{
    BlockFilter,
    BlockOrder,
    CanonData,
    DigestTree,
    FixMode,
    Peer,
    SerialRecord,
    SerialTransaction,
    SyncStorage,
    ValidatorError,
};

use super::*;

fn read_static_blob<const S: usize>(row: &Row, index: usize) -> rusqlite::Result<[u8; S]> {
    (&row.get::<_, Vec<u8>>(index)?[..])
        .try_into()
        .map_err(|_| rusqlite::Error::InvalidQuery)
}

impl SqliteStorage {
    /// Counter used for tracking migrations, incremented on each schema change, and checked in [`migrate`] function below to update schema.
    const SCHEMA_INDEX: u32 = 3;

    fn migrate(&self, from: u32) -> Result<()> {
        if from == 0 {
            self.conn.execute_batch(r"
            CREATE TABLE IF NOT EXISTS blocks(
                id INTEGER PRIMARY KEY,
                canon_height INTEGER,
                canon_ledger_digest BLOB,
                hash BLOB UNIQUE NOT NULL,
                previous_block_id INTEGER, -- REFERENCES blocks(id) ON DELETE SET NULL -- can't do cyclic fk ref in sqlite
                previous_block_hash BLOB NOT NULL,
                merkle_root_hash BLOB NOT NULL,
                pedersen_merkle_root_hash BLOB NOT NULL,
                proof BLOB NOT NULL,
                time INTEGER NOT NULL,
                difficulty_target INTEGER NOT NULL,
                nonce INTEGER NOT NULL
            );
            CREATE INDEX previous_block_id_lookup ON blocks(previous_block_id);
            CREATE INDEX previous_block_hash_lookup ON blocks(previous_block_hash);
            CREATE INDEX canon_height_lookup ON blocks(canon_height);

            CREATE TABLE IF NOT EXISTS transactions(
                id INTEGER PRIMARY KEY,
                transaction_id BLOB UNIQUE NOT NULL,
                network INTEGER NOT NULL,
                ledger_digest BLOB NOT NULL,
                old_serial_number1 BLOB NOT NULL,
                old_serial_number2 BLOB NOT NULL,
                new_commitment1 BLOB NOT NULL,
                new_commitment2 BLOB NOT NULL,
                program_commitment BLOB NOT NULL,
                local_data_root BLOB NOT NULL,
                value_balance INTEGER NOT NULL,
                signature1 BLOB NOT NULL,
                signature2 BLOB NOT NULL,
                new_record1 BLOB NOT NULL,
                new_record2 BLOB NOT NULL,
                proof BLOB NOT NULL,
                memo BLOB NOT NULL,
                inner_circuit_id BLOB NOT NULL
            );

            CREATE TABLE IF NOT EXISTS transaction_blocks(
                id INTEGER PRIMARY KEY,
                transaction_id INTEGER NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
                block_id INTEGER NOT NULL REFERENCES blocks(id) ON DELETE CASCADE,
                block_order INTEGER NOT NULL
            );
            CREATE UNIQUE INDEX transaction_block_ordering ON transaction_blocks(block_id, block_order);
            CREATE INDEX transaction_block_lookup ON transaction_blocks(transaction_id);
            ")?;
        }

        if from <= 1 {
            self.conn.execute_batch(
                r"
            CREATE TABLE IF NOT EXISTS miner_records(
                id INTEGER PRIMARY KEY,
                owner TEXT NOT NULL,
                is_dummy INTEGER NOT NULL,
                value INTEGER NOT NULL,
                payload BLOB NOT NULL,
                birth_program_id BLOB NOT NULL,
                death_program_id BLOB NOT NULL,
                serial_number_nonce BLOB NOT NULL,
                commitment BLOB NOT NULL,
                commitment_randomness BLOB NOT NULL
            );
            CREATE INDEX record_owner_lookup ON miner_records(owner);
            CREATE INDEX record_commitment_lookup ON miner_records(commitment);
            ",
            )?;
        }

        if from <= 2 {
            self.conn.execute_batch(
                r"
            CREATE INDEX blocks_time_lookup ON blocks(time);
            CREATE TABLE IF NOT EXISTS peers(
                id INTEGER PRIMARY KEY,
                address TEXT NOT NULL,
                block_height INTEGER NOT NULL,
                first_seen INTEGER,
                last_seen INTEGER,
                last_connected INTEGER,
                blocks_synced_to INTEGER NOT NULL,
                blocks_synced_from INTEGER NOT NULL,
                blocks_received_from INTEGER NOT NULL,
                blocks_sent_to INTEGER NOT NULL,
                connection_attempt_count INTEGER NOT NULL,
                connection_success_count INTEGER NOT NULL,
                connection_transient_fail_count INTEGER NOT NULL
            );
            CREATE UNIQUE INDEX peer_address_lookup ON peers(address);
            CREATE INDEX peer_last_seen_lookup ON peers(last_seen);
            ",
            )?;
        }

        if from == 0 {
            self.conn
                .execute("INSERT INTO migration VALUES (?)", [Self::SCHEMA_INDEX])?;
        } else {
            self.conn
                .execute("UPDATE migration SET schema_index = ?", [Self::SCHEMA_INDEX])?;
        }
        Ok(())
    }
}

impl SyncStorage for SqliteStorage {
    fn init(&mut self) -> Result<()> {
        self.conn.execute(
            r"
        CREATE TABLE IF NOT EXISTS migration(
            schema_index INTEGER PRIMARY KEY
        );",
            [],
        )?;
        let current_index = self
            .conn
            .query_row(
                r"
        SELECT schema_index FROM migration LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(0u32);
        if current_index < Self::SCHEMA_INDEX {
            self.migrate(current_index)?;
        }
        self.conn.execute(r"PRAGMA OPTIMIZE;", [])?;
        self.conn.query_row(r"PRAGMA journal_mode=WAL;", [], |_| Ok(()))?;
        Ok(())
    }

    fn insert_block(&mut self, block: &SerialBlock) -> Result<()> {
        self.optimize()?;
        let hash = block.header.hash();

        match self.get_block_state(&hash)? {
            BlockStatus::Unknown => (),
            BlockStatus::Committed(_) | BlockStatus::Uncommitted => {
                metrics::increment_counter!(snarkos_metrics::blocks::DUPLICATES);
                return Err(anyhow!("duplicate block insertion"));
            }
        }

        let mut block_query = self.conn.prepare_cached(
            r"
        INSERT INTO blocks (
            hash,
            previous_block_id,
            previous_block_hash,
            merkle_root_hash,
            pedersen_merkle_root_hash,
            proof,
            time,
            difficulty_target,
            nonce)
            VALUES (
                ?,
                (SELECT id from blocks where hash = ?),
                ?,
                ?,
                ?,
                ?,
                ?,
                ?,
                ?
            )
        ",
        )?;
        block_query.execute::<&[&dyn ToSql]>(&[
            &hash,
            &block.header.previous_block_hash,
            &block.header.previous_block_hash,
            &&block.header.merkle_root_hash.0[..],
            &&block.header.pedersen_merkle_root_hash.0[..],
            &&block.header.proof.0[..],
            &block.header.time,
            &block.header.difficulty_target,
            &block.header.nonce,
        ])?;
        let block_id = self.conn.last_insert_rowid();
        self.conn.execute(
            "UPDATE blocks SET previous_block_id = ? WHERE previous_block_hash = ?",
            params![block_id, hash],
        )?;
        let mut transaction_query = self.conn.prepare_cached(
            r"
            INSERT OR IGNORE INTO transactions (
                transaction_id,
                network,
                ledger_digest,
                old_serial_number1,
                old_serial_number2,
                new_commitment1,
                new_commitment2,
                program_commitment,
                local_data_root,
                value_balance,
                signature1,
                signature2,
                new_record1,
                new_record2,
                proof,
                memo,
                inner_circuit_id
            )
            VALUES (
                ?,
                ?,
                ?,
                ?,
                ?,
                ?,
                ?,
                ?,
                ?,
                ?,
                ?,
                ?,
                ?,
                ?,
                ?,
                ?,
                ?
            )
        ",
        )?;
        let mut transaction_block_query = self.conn.prepare_cached(
            r"
            INSERT INTO transaction_blocks (
                transaction_id,
                block_id,
                block_order
            )
            VALUES (
                (SELECT id FROM transactions WHERE transaction_id = ?),
                ?,
                ?
            )
        ",
        )?;
        for (i, transaction) in block.transactions.iter().enumerate() {
            transaction_query.execute(params![
                &transaction.id[..],
                transaction.network.id(),
                transaction.ledger_digest,
                &transaction.old_serial_numbers[0],
                &transaction.old_serial_numbers[1],
                &transaction.new_commitments[0],
                &transaction.new_commitments[1],
                transaction.program_commitment,
                transaction.local_data_root,
                transaction.value_balance.0,
                &transaction.signatures[0],
                &transaction.signatures[1],
                &transaction.new_records[0],
                &transaction.new_records[1],
                &transaction.transaction_proof[..],
                transaction.memorandum,
                transaction.inner_circuit_id,
            ])?;
            transaction_block_query.execute(params![&transaction.id[..], block_id as usize, i])?;
        }
        Ok(())
    }

    fn delete_block(&mut self, hash: &Digest) -> Result<()> {
        self.optimize()?;

        self.conn.execute(
            r"
            DELETE FROM blocks WHERE hash = ?
        ",
            [hash],
        )?;

        // clean messy sqlite fk constraints
        self.conn.execute(
            r"
            DELETE FROM transaction_blocks
            WHERE id IN (
                SELECT tb.id FROM transaction_blocks tb
                LEFT JOIN blocks b ON b.id = tb.block_id WHERE b.id IS NULL
            );
        ",
            [],
        )?;

        self.conn.execute(
            r"
            DELETE FROM transactions
            WHERE id IN (
                SELECT t.id FROM transactions t
                LEFT JOIN transaction_blocks tb ON tb.transaction_id = t.id WHERE tb.id IS NULL
            );
        ",
            [],
        )?;
        Ok(())
    }

    fn get_block_hash(&mut self, block_num: u32) -> Result<Option<Digest>> {
        self.optimize()?;

        Ok(self
            .conn
            .query_row::<Vec<u8>, _, _>(r"SELECT hash FROM blocks WHERE canon_height = ?", [block_num], |row| {
                row.get(0)
            })
            .optional()?
            .map(|x| Digest::from(&x[..])))
    }

    fn get_block_header(&mut self, hash: &Digest) -> Result<SerialBlockHeader> {
        self.optimize()?;

        self.conn
            .query_row(
                r"
            SELECT
                previous_block_hash,
                merkle_root_hash,
                pedersen_merkle_root_hash,
                proof,
                time,
                difficulty_target,
                nonce
            FROM blocks WHERE hash = ?",
                [hash],
                |row| {
                    Ok(SerialBlockHeader {
                        previous_block_hash: row.get(0)?,
                        merkle_root_hash: MerkleRootHash(read_static_blob(row, 1)?),
                        pedersen_merkle_root_hash: PedersenMerkleRootHash(read_static_blob(row, 2)?),
                        proof: ProofOfSuccinctWork(read_static_blob(row, 3)?),
                        time: row.get(4)?,
                        difficulty_target: row.get(5)?,
                        nonce: row.get(6)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    fn get_block_state(&mut self, hash: &Digest) -> Result<BlockStatus> {
        self.optimize()?;

        let output: Option<Option<usize>> = self
            .conn
            .query_row(r"SELECT canon_height FROM blocks WHERE hash = ?", [hash], |row| {
                row.get(0)
            })
            .optional()?;

        Ok(match output {
            None => BlockStatus::Unknown,
            Some(None) => BlockStatus::Uncommitted,
            Some(Some(n)) => BlockStatus::Committed(n),
        })
    }

    fn get_block_states(&mut self, hashes: &[Digest]) -> Result<Vec<BlockStatus>> {
        self.optimize()?;

        // intentional N+1 query since rusqlite doesn't support WHERE ... IN here and it doesn't matter at the moment
        let mut out = Vec::with_capacity(hashes.len());
        for hash in hashes {
            let state = self.get_block_state(hash)?;
            out.push(state);
        }
        Ok(out)
    }

    fn get_block(&mut self, hash: &Digest) -> Result<SerialBlock> {
        self.optimize()?;

        let header = self.get_block_header(hash)?;
        let mut stmt = self.conn.prepare_cached(
            "SELECT
            transactions.transaction_id,
            network,
            ledger_digest,
            old_serial_number1,
            old_serial_number2,
            new_commitment1,
            new_commitment2,
            program_commitment,
            local_data_root,
            value_balance,
            signature1,
            signature2,
            new_record1,
            new_record2,
            transactions.proof,
            memo,
            inner_circuit_id
        FROM transactions
        INNER JOIN transaction_blocks on transaction_blocks.transaction_id = transactions.id
        INNER JOIN blocks on blocks.id = transaction_blocks.block_id
        WHERE blocks.hash = ?
        ORDER BY transaction_blocks.block_order ASC",
        )?;
        let rows = stmt.query_map([hash], |row| {
            Ok(SerialTransaction {
                id: read_static_blob(row, 0)?,
                network: Network::from_id(row.get(1)?),
                ledger_digest: row.get(2)?,
                old_serial_numbers: vec![row.get(3)?, row.get(4)?],
                new_commitments: vec![row.get(5)?, row.get(6)?],
                program_commitment: row.get(7)?,
                local_data_root: row.get(8)?,
                value_balance: AleoAmount(row.get(9)?),
                signatures: vec![row.get(10)?, row.get(11)?],
                new_records: vec![row.get(12)?, row.get(13)?],
                transaction_proof: row.get(14)?,
                memorandum: row.get(15)?,
                inner_circuit_id: row.get(16)?,
            })
        })?;
        Ok(SerialBlock {
            header,
            transactions: rows.collect::<rusqlite::Result<_>>()?,
        })
    }

    fn commit_block(&mut self, hash: &Digest, ledger_digest: &Digest) -> Result<BlockStatus> {
        self.optimize()?;

        let canon = self.canon()?;
        match self.get_block_state(hash)? {
            BlockStatus::Committed(_) => {
                return Err(anyhow!("attempted to recommit block {}", hex::encode(hash)));
            }
            BlockStatus::Unknown => return Err(anyhow!("attempted to commit unknown block")),
            _ => (),
        }
        let next_canon_height = if canon.is_empty() { 0 } else { canon.block_height + 1 };
        self.conn.execute(
            r"UPDATE blocks SET canon_height = ?, canon_ledger_digest = ? WHERE hash = ?",
            params![next_canon_height, ledger_digest, hash],
        )?;
        self.get_block_state(hash)
    }

    fn recommit_blockchain(&mut self, root_hash: &Digest) -> Result<()> {
        let canon = self.canon()?;
        match self.get_block_state(root_hash)? {
            BlockStatus::Committed(_) => {
                return Err(anyhow!("attempted to recommit block {}", hex::encode(root_hash)));
            }
            BlockStatus::Unknown => return Err(anyhow!("attempted to commit unknown block")),
            _ => (),
        }
        let next_canon_height = if canon.is_empty() { 0 } else { canon.block_height + 1 };
        self.conn.execute(
            r"
            WITH RECURSIVE
                children(parent, sub, length) AS (
                    SELECT ?, NULL, 0 as length
                    UNION ALL
                    SELECT blocks.hash, blocks.previous_block_hash, children.length + 1 FROM blocks
                    INNER JOIN children
                    WHERE blocks.previous_block_hash = children.parent
                ),
                preferred_tip AS (
                    SELECT parent, sub, length FROM children
                    WHERE length = (SELECT max(length) FROM children)
                    ORDER BY parent
                    LIMIT 1
                ),
                total_tip(parent, remaining, digest) AS (
                    SELECT preferred_tip.parent, preferred_tip.length, NULL FROM preferred_tip
                    UNION ALL
                    SELECT blocks.previous_block_hash, total_tip.remaining - 1, blocks.canon_ledger_digest
                    FROM total_tip
                    INNER JOIN blocks ON blocks.hash = total_tip.parent
                )
                UPDATE blocks SET
                    canon_height = total_tip.remaining + ?
                FROM total_tip
                WHERE
                    total_tip.parent = blocks.hash
                    AND total_tip.digest IS NOT NULL;
            ",
            params![root_hash, next_canon_height],
        )?;
        Ok(())
    }

    fn recommit_block(&mut self, hash: &Digest) -> Result<BlockStatus> {
        let canon = self.canon()?;
        match self.get_block_state(hash)? {
            BlockStatus::Committed(_) => {
                return Err(anyhow!("attempted to recommit block {}", hex::encode(hash)));
            }
            BlockStatus::Unknown => return Err(anyhow!("attempted to commit unknown block")),
            _ => (),
        }
        let next_canon_height = if canon.is_empty() { 0 } else { canon.block_height + 1 };
        self.conn.execute(
            r"UPDATE blocks SET canon_height = ? WHERE hash = ? AND canon_ledger_digest IS NOT NULL",
            params![next_canon_height, hash],
        )?;
        self.get_block_state(hash)
    }

    fn decommit_blocks(&mut self, hash: &Digest) -> Result<Vec<SerialBlock>> {
        self.optimize()?;

        match self.get_block_state(hash)? {
            BlockStatus::Committed(_) => (),
            _ => return Err(anyhow!("attempted to decommit uncommitted block")),
        }
        let canon = self.canon()?;
        if canon.block_height == 0 {
            return Err(anyhow!("cannot decommit genesis block"));
        }
        let mut decommitted = vec![];

        let mut last_hash = canon.hash;
        loop {
            let block = self.get_block(&last_hash)?;
            let block_number = match self.get_block_state(&last_hash)? {
                BlockStatus::Unknown => return Err(anyhow!("unknown block state")),
                BlockStatus::Committed(n) => n as u32,
                BlockStatus::Uncommitted => return Err(anyhow!("uncommitted block in decommit")),
            };

            debug!("Decommitting block {} ({})", last_hash, block_number);

            self.conn
                .execute(r"UPDATE blocks SET canon_height = NULL WHERE hash = ?", [&last_hash])?;

            let new_last_hash = block.header.previous_block_hash.clone();
            decommitted.push(block);
            if &last_hash == hash {
                break;
            }
            last_hash = new_last_hash;
        }

        Ok(decommitted)
    }

    fn canon_height(&mut self) -> Result<u32> {
        self.optimize()?;

        self.conn
            .query_row(r"SELECT coalesce(max(canon_height), 0) FROM blocks", [], |row| {
                row.get(0)
            })
            .map_err(Into::into)
    }

    fn canon(&mut self) -> Result<CanonData> {
        self.optimize()?;

        let canon_height = self.canon_height()?;

        let hash = self.get_block_hash(canon_height)?;
        // handle genesis
        if hash.is_none() && canon_height == 0 {
            return Ok(CanonData {
                block_height: 0,
                hash: Digest::default(), // empty
            });
        }
        Ok(CanonData {
            block_height: canon_height as usize,
            hash: hash.ok_or_else(|| anyhow!("missing canon block"))?,
        })
    }

    fn longest_child_path(&mut self, block_hash: &Digest) -> Result<Vec<Digest>> {
        self.optimize()?;

        let mut stmt = self.conn.prepare_cached(
            r"
            WITH RECURSIVE
                children(parent, sub, length) AS (
                    SELECT ?, NULL, 0 as length
                    UNION ALL
                    SELECT blocks.hash, blocks.previous_block_hash, children.length + 1 FROM blocks
                    INNER JOIN children
                    WHERE blocks.previous_block_hash = children.parent
                ),
                preferred_tip AS (
                    SELECT parent, sub, length FROM children
                    WHERE length = (SELECT max(length) FROM children)
                    ORDER BY parent
                    LIMIT 1
                ),
                total_tip(parent, remaining) AS (
                    SELECT preferred_tip.parent, preferred_tip.length FROM preferred_tip
                    UNION ALL
                    SELECT blocks.previous_block_hash, total_tip.remaining - 1
                    FROM total_tip
                    INNER JOIN blocks ON blocks.hash = total_tip.parent
                    WHERE total_tip.remaining > 0
                )
                SELECT total_tip.parent, total_tip.remaining FROM total_tip
                order by remaining;
        ",
        )?;
        let out = stmt
            .query_map([block_hash], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<Digest>>>()?;
        Ok(out)
    }

    fn get_block_digest_tree(&mut self, block_hash: &Digest) -> Result<DigestTree> {
        self.optimize()?;

        let mut stmt = self.conn.prepare_cached(
            r"
            WITH RECURSIVE
                children(parent, sub, length) AS (
                    SELECT ?, NULL, 0 as length
                    UNION ALL
                    SELECT blocks.hash, blocks.previous_block_hash, children.length + 1 FROM blocks
                    INNER JOIN children
                    WHERE blocks.previous_block_hash = children.parent
                )
                SELECT * FROM children
                WHERE length > 0
                ORDER BY length;
        ",
        )?;
        let out = stmt
            .query_map([block_hash], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<rusqlite::Result<Vec<(Digest, Digest, u32)>>>()?;

        let mut past_leaves = HashedMap::<Digest, Vec<DigestTree>>::default();
        let mut pending_leaves = HashedMap::<Digest, Vec<DigestTree>>::default();
        let mut current_tree_depth = None::<u32>;
        for (hash, parent_hash, tree_depth) in out.into_iter().rev() {
            if current_tree_depth.is_none() {
                current_tree_depth = Some(tree_depth);
            } else if Some(tree_depth) != current_tree_depth {
                current_tree_depth = Some(tree_depth);

                past_leaves.clear();
                std::mem::swap(&mut past_leaves, &mut pending_leaves);
            }
            let waiting_children = past_leaves.remove(&hash).unwrap_or_default();
            let node = if !waiting_children.is_empty() {
                let max_dist = waiting_children.iter().map(|x| x.longest_length()).max().unwrap_or(0);
                DigestTree::Node(hash, waiting_children, max_dist)
            } else {
                DigestTree::Leaf(hash)
            };
            pending_leaves.entry(parent_hash).or_insert_with(Vec::new).push(node);
        }

        if let Some(children) = pending_leaves.remove(block_hash) {
            let max_dist = children.iter().map(|x| x.longest_length()).max().unwrap_or(0);

            Ok(DigestTree::Node(block_hash.clone(), children, max_dist))
        } else {
            Ok(DigestTree::Leaf(block_hash.clone()))
        }
    }

    fn get_block_children(&mut self, hash: &Digest) -> Result<Vec<Digest>> {
        self.optimize()?;

        let mut stmt = self.conn.prepare_cached(
            r"
            SELECT blocks.hash FROM blocks
            WHERE blocks.previous_block_hash = ?
            ORDER BY blocks.hash
        ",
        )?;
        let out = stmt
            .query_map([hash], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<Digest>>>()?;
        Ok(out)
    }

    fn scan_forks(&mut self, scan_depth: u32) -> Result<Vec<(Digest, Digest)>> {
        self.optimize()?;

        let mut stmt = self.conn.prepare_cached(
            r"
            WITH RECURSIVE
                children(parent, sub, length) AS (
                    SELECT NULL, (select hash from blocks where canon_height = (select max(canon_height) from blocks)), 0
                    UNION ALL
                    SELECT blocks.hash, blocks.previous_block_hash, children.length + 1 FROM blocks
                    INNER JOIN children
                    WHERE blocks.hash = children.sub AND blocks.canon_height IS NOT NULL AND length <= ?
                )
                SELECT b.previous_block_hash, b.hash FROM children
                INNER JOIN blocks b ON b.previous_block_hash = children.sub AND b.hash != children.parent
                WHERE children.length > 0
                GROUP BY children.parent
                HAVING count(b.id) >= 1
                ORDER BY length;
        ",
        )?;

        let out = stmt
            .query_map([scan_depth], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<Vec<(Digest, Digest)>>>()?;
        Ok(out)
    }

    fn get_transaction_location(&mut self, transaction_id: &Digest) -> Result<Option<TransactionLocation>> {
        self.optimize()?;

        self.conn
            .query_row(
                r"
        SELECT
        transaction_blocks.block_order,
        blocks.hash
        FROM transactions
        INNER JOIN transaction_blocks ON transaction_blocks.transaction_id = transactions.id
        INNER JOIN blocks ON blocks.id = transaction_blocks.block_id
        WHERE transactions.transaction_id = ? AND blocks.canon_height IS NOT NULL
        ",
                [transaction_id],
                |row| {
                    Ok(TransactionLocation {
                        index: row.get(0)?,
                        block_hash: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    fn get_transaction(&mut self, transaction_id: &Digest) -> Result<SerialTransaction> {
        self.optimize()?;

        self.conn
            .query_row(
                r"
        SELECT
            transactions.transaction_id,
            network,
            ledger_digest,
            old_serial_number1,
            old_serial_number2,
            new_commitment1,
            new_commitment2,
            program_commitment,
            local_data_root,
            value_balance,
            signature1,
            signature2,
            new_record1,
            new_record2,
            proof,
            memo,
            inner_circuit_id
        FROM transactions
        WHERE transactions.transaction_id = ?
        ",
                [transaction_id],
                |row| {
                    Ok(SerialTransaction {
                        id: read_static_blob(row, 0)?,
                        network: Network::from_id(row.get(1)?),
                        ledger_digest: row.get(2)?,
                        old_serial_numbers: vec![row.get(3)?, row.get(4)?],
                        new_commitments: vec![row.get(5)?, row.get(6)?],
                        program_commitment: row.get(7)?,
                        local_data_root: row.get(8)?,
                        value_balance: AleoAmount(row.get(9)?),
                        signatures: vec![row.get(10)?, row.get(11)?],
                        new_records: vec![row.get(12)?, row.get(13)?],
                        transaction_proof: row.get(14)?,
                        memorandum: row.get(15)?,
                        inner_circuit_id: row.get(16)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    fn get_record_commitments(&mut self, limit: Option<usize>) -> Result<Vec<Digest>> {
        self.optimize()?;

        let mut stmt = self.conn.prepare_cached(
            r"
        SELECT commitment
        FROM miner_records
        LIMIT ?
        ",
        )?;
        let digests = stmt
            .query_map([(limit.map(|x| x as u32)).unwrap_or(u32::MAX)], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<Digest>>>()?;
        Ok(digests)
    }

    fn get_record(&mut self, commitment: &Digest) -> Result<Option<SerialRecord>> {
        self.conn
            .query_row(
                r"
        SELECT
            owner,
            is_dummy,
            value,
            payload,
            birth_program_id,
            death_program_id,
            serial_number_nonce,
            commitment,
            commitment_randomness
        FROM miner_records
        WHERE commitment = ?
        ",
                [commitment],
                |row| {
                    Ok(SerialRecord {
                        owner: row
                            .get::<_, String>(0)?
                            .parse()
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        is_dummy: row.get(1)?,
                        value: AleoAmount(row.get(2)?),
                        payload: row.get(3)?,
                        birth_program_id: row.get(4)?,
                        death_program_id: row.get(5)?,
                        serial_number_nonce: row.get(6)?,
                        commitment: row.get(7)?,
                        commitment_randomness: row.get(8)?,
                        serial_number_nonce_randomness: None,
                        position: None,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    fn store_records(&mut self, records: &[SerialRecord]) -> Result<()> {
        self.optimize()?;

        let mut stmt = self.conn.prepare_cached(
            r"
        INSERT INTO miner_records (
            owner,
            is_dummy,
            value,
            payload,
            birth_program_id,
            death_program_id,
            serial_number_nonce,
            commitment,
            commitment_randomness
        ) VALUES (
            ?, ?, ?, ?, ?, ?, ?, ?, ?
        )
        ",
        )?;
        for record in records {
            stmt.execute(params![
                record.owner.to_string(),
                record.is_dummy,
                record.value.0,
                &record.payload,
                &record.birth_program_id,
                &record.death_program_id,
                &record.serial_number_nonce,
                &record.commitment,
                &record.commitment_randomness,
            ])?;
        }
        Ok(())
    }

    fn get_commitments(&mut self, block_start: u32) -> Result<Vec<Digest>> {
        self.optimize()?;

        let mut stmt = self.conn.prepare_cached(
            r"
        SELECT
        transactions.new_commitment1,
        transactions.new_commitment2
        FROM transactions
        INNER JOIN transaction_blocks ON transaction_blocks.transaction_id = transactions.id
        INNER JOIN blocks ON blocks.id = transaction_blocks.block_id
        WHERE blocks.canon_height IS NOT NULL AND blocks.canon_height >= ?
        ORDER BY blocks.canon_height ASC, transaction_blocks.block_order ASC
        ",
        )?;
        let digests = stmt
            .query_map([block_start], |row| Ok([row.get(0)?, row.get(1)?]))?
            .collect::<rusqlite::Result<Vec<[Digest; 2]>>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok(digests)
    }

    fn get_serial_numbers(&mut self, block_start: u32) -> Result<Vec<Digest>> {
        self.optimize()?;

        let mut stmt = self.conn.prepare_cached(
            r"
        SELECT
        transactions.old_serial_number1,
        transactions.old_serial_number2
        FROM transactions
        INNER JOIN transaction_blocks ON transaction_blocks.transaction_id = transactions.id
        INNER JOIN blocks ON blocks.id = transaction_blocks.block_id
        WHERE blocks.canon_height IS NOT NULL AND blocks.canon_height >= ?
        ORDER BY blocks.canon_height ASC, transaction_blocks.block_order ASC
        ",
        )?;
        let digests = stmt
            .query_map([block_start], |row| Ok([row.get(0)?, row.get(1)?]))?
            .collect::<rusqlite::Result<Vec<[Digest; 2]>>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok(digests)
    }

    fn get_memos(&mut self, block_start: u32) -> Result<Vec<Digest>> {
        self.optimize()?;

        let mut stmt = self.conn.prepare_cached(
            r"
        SELECT
        transactions.memo
        FROM transactions
        INNER JOIN transaction_blocks ON transaction_blocks.transaction_id = transactions.id
        INNER JOIN blocks ON blocks.id = transaction_blocks.block_id
        WHERE blocks.canon_height IS NOT NULL AND blocks.canon_height >= ?
        ORDER BY blocks.canon_height ASC, transaction_blocks.block_order ASC
        ",
        )?;
        let digests = stmt
            .query_map([block_start], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<Digest>>>()?;
        Ok(digests)
    }

    fn get_ledger_digests(&mut self, block_start: u32) -> Result<Vec<Digest>> {
        self.optimize()?;

        let mut stmt = self.conn.prepare_cached(
            r"
        SELECT
        blocks.canon_ledger_digest
        FROM blocks
        WHERE blocks.canon_height IS NOT NULL AND blocks.canon_height >= ?
        ORDER BY blocks.canon_height ASC
        ",
        )?;
        let digests = stmt
            .query_map([block_start], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<Digest>>>()?;
        Ok(digests)
    }

    fn reset_ledger(
        &mut self,
        _commitments: Vec<Digest>,
        _serial_numbers: Vec<Digest>,
        _memos: Vec<Digest>,
        _digests: Vec<Digest>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn get_canon_blocks(&mut self, limit: Option<u32>) -> Result<Vec<SerialBlock>> {
        self.optimize()?;

        let digests = self.get_block_hashes(limit, BlockFilter::CanonOnly(BlockOrder::Unordered))?;
        // this is intentionally N+1 query since this is not a critical performance function and its easy
        let mut blocks = Vec::with_capacity(digests.len());
        for digest in digests {
            blocks.push(self.get_block(&digest)?);
        }

        Ok(blocks)
    }

    fn get_block_hashes(&mut self, limit: Option<u32>, filter: BlockFilter) -> Result<Vec<Digest>> {
        self.optimize()?;

        let limit = limit.unwrap_or(u32::MAX);
        let hashes = match filter {
            BlockFilter::CanonOnly(BlockOrder::Unordered) => {
                let mut stmt = self.conn.prepare_cached(
                    "
                    SELECT
                    blocks.hash
                    FROM blocks
                    WHERE blocks.canon_height IS NOT NULL
                    LIMIT ?
                ",
                )?;
                let digests = stmt
                    .query_map([limit], |row| row.get(0))?
                    .collect::<rusqlite::Result<Vec<Digest>>>()?;
                digests
            }
            BlockFilter::CanonOnly(BlockOrder::Ascending) => {
                let mut stmt = self.conn.prepare_cached(
                    "
                    SELECT
                    blocks.hash
                    FROM blocks
                    WHERE blocks.canon_height IS NOT NULL
                    ORDER BY blocks.canon_height ASC
                    LIMIT ?
                ",
                )?;
                let digests = stmt
                    .query_map([limit], |row| row.get(0))?
                    .collect::<rusqlite::Result<Vec<Digest>>>()?;
                digests
            }
            BlockFilter::CanonOnly(BlockOrder::Descending) => {
                let mut stmt = self.conn.prepare_cached(
                    "
                    SELECT
                    blocks.hash
                    FROM blocks
                    WHERE blocks.canon_height IS NOT NULL
                    ORDER BY blocks.canon_height DESC
                    LIMIT ?
                ",
                )?;
                let digests = stmt
                    .query_map([limit], |row| row.get(0))?
                    .collect::<rusqlite::Result<Vec<Digest>>>()?;
                digests
            }
            BlockFilter::NonCanonOnly => {
                let mut stmt = self.conn.prepare_cached(
                    "
                    SELECT
                    blocks.hash
                    FROM blocks
                    WHERE blocks.canon_height IS NULL
                    LIMIT ?
                ",
                )?;
                let digests = stmt
                    .query_map([limit], |row| row.get(0))?
                    .collect::<rusqlite::Result<Vec<Digest>>>()?;
                digests
            }
            BlockFilter::All => {
                let mut stmt = self.conn.prepare_cached(
                    "
                    SELECT
                    blocks.hash
                    FROM blocks
                    LIMIT ?
                ",
                )?;
                let digests = stmt
                    .query_map([limit], |row| row.get(0))?
                    .collect::<rusqlite::Result<Vec<Digest>>>()?;
                digests
            }
        };
        Ok(hashes)
    }

    fn store_peers(&mut self, peers: Vec<Peer>) -> Result<()> {
        self.optimize()?;

        let mut stmt = self.conn.prepare_cached(
            "
            INSERT INTO peers (
                address,
                block_height,
                first_seen,
                last_seen,
                last_connected,
                blocks_synced_to,
                blocks_synced_from,
                blocks_received_from,
                blocks_sent_to,
                connection_attempt_count,
                connection_success_count,
                connection_transient_fail_count
            )
            VALUES (
                ?,
                ?,
                ?,
                ?,
                ?,
                ?,
                ?,
                ?,
                ?,
                ?,
                ?,
                ?
            )
            ON CONFLICT(address)
            DO UPDATE SET
                block_height = excluded.block_height,
                first_seen = excluded.first_seen,
                last_seen = excluded.last_seen,
                last_connected = excluded.last_connected,
                blocks_synced_to = excluded.blocks_synced_to,
                blocks_synced_from = excluded.blocks_synced_from,
                blocks_received_from = excluded.blocks_received_from,
                blocks_sent_to = excluded.blocks_sent_to,
                connection_attempt_count = excluded.connection_attempt_count,
                connection_success_count = excluded.connection_success_count,
                connection_transient_fail_count = excluded.connection_transient_fail_count
        ",
        )?;

        for peer in peers {
            stmt.execute(params![
                peer.address.to_string(),
                peer.block_height,
                peer.first_seen.map(|x| x.naive_utc().timestamp()),
                peer.last_seen.map(|x| x.naive_utc().timestamp()),
                peer.last_connected.map(|x| x.naive_utc().timestamp()),
                peer.blocks_synced_to,
                peer.blocks_synced_from,
                peer.blocks_received_from,
                peer.blocks_sent_to,
                peer.connection_attempt_count,
                peer.connection_success_count,
                peer.connection_transient_fail_count,
            ])?;
        }
        Ok(())
    }

    fn lookup_peers(&mut self, addresses: Vec<SocketAddr>) -> Result<Vec<Option<Peer>>> {
        self.optimize()?;

        let mut out = vec![];
        let mut stmt = self.conn.prepare_cached(
            "
            SELECT
                block_height,
                first_seen,
                last_seen,
                last_connected,
                blocks_synced_to,
                blocks_synced_from,
                blocks_received_from,
                blocks_sent_to,
                connection_attempt_count,
                connection_success_count,
                connection_transient_fail_count
            FROM peers
            WHERE address = ?
        ",
        )?;
        // todo: this is O(n) queries, but this isn't large-size input and convenient
        for address in addresses {
            let peer = stmt
                .query_row([address.to_string()], |row| {
                    Ok(Peer {
                        address,
                        block_height: row.get(0)?,
                        first_seen: row
                            .get::<_, Option<i64>>(1)?
                            .map(|x| DateTime::from_utc(NaiveDateTime::from_timestamp(x, 0), Utc)),
                        last_seen: row
                            .get::<_, Option<i64>>(2)?
                            .map(|x| DateTime::from_utc(NaiveDateTime::from_timestamp(x, 0), Utc)),
                        last_connected: row
                            .get::<_, Option<i64>>(3)?
                            .map(|x| DateTime::from_utc(NaiveDateTime::from_timestamp(x, 0), Utc)),
                        blocks_synced_to: row.get(4)?,
                        blocks_synced_from: row.get(5)?,
                        blocks_received_from: row.get(6)?,
                        blocks_sent_to: row.get(7)?,
                        connection_attempt_count: row.get(8)?,
                        connection_success_count: row.get(9)?,
                        connection_transient_fail_count: row.get(10)?,
                    })
                })
                .optional()?;
            out.push(peer);
        }
        Ok(out)
    }

    fn fetch_peers(&mut self) -> Result<Vec<Peer>> {
        self.optimize()?;

        let mut stmt = self.conn.prepare_cached(
            "
            SELECT
                address,
                block_height,
                first_seen,
                last_seen,
                last_connected,
                blocks_synced_to,
                blocks_synced_from,
                blocks_received_from,
                blocks_sent_to,
                connection_attempt_count,
                connection_success_count,
                connection_transient_fail_count
            FROM peers
        ",
        )?;

        let query = stmt.query_map([], |row| {
            Ok(Peer {
                address: row
                    .get::<_, String>(0)?
                    .parse()
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                block_height: row.get(1)?,
                first_seen: row
                    .get::<_, Option<i64>>(2)?
                    .map(|x| DateTime::from_utc(NaiveDateTime::from_timestamp(x, 0), Utc)),
                last_seen: row
                    .get::<_, Option<i64>>(3)?
                    .map(|x| DateTime::from_utc(NaiveDateTime::from_timestamp(x, 0), Utc)),
                last_connected: row
                    .get::<_, Option<i64>>(4)?
                    .map(|x| DateTime::from_utc(NaiveDateTime::from_timestamp(x, 0), Utc)),
                blocks_synced_to: row.get(5)?,
                blocks_synced_from: row.get(6)?,
                blocks_received_from: row.get(7)?,
                blocks_sent_to: row.get(8)?,
                connection_attempt_count: row.get(9)?,
                connection_success_count: row.get(10)?,
                connection_transient_fail_count: row.get(11)?,
            })
        })?;

        Ok(query.collect::<Result<Vec<_>, rusqlite::Error>>()?)
    }

    fn validate(&mut self, _limit: Option<u32>, _fix_mode: FixMode) -> Vec<ValidatorError> {
        warn!("called validator on sqlite, which is a NOP");
        vec![]
    }

    #[cfg(feature = "test")]
    fn store_item(&mut self, _col: KeyValueColumn, _key: Vec<u8>, _value: Vec<u8>) -> Result<()> {
        unimplemented!()
    }

    #[cfg(feature = "test")]
    fn delete_item(&mut self, _col: KeyValueColumn, _key: Vec<u8>) -> Result<()> {
        unimplemented!()
    }

    fn transact<T, F: FnOnce(&mut Self) -> Result<T>>(&mut self, func: F) -> Result<T> {
        self.conn.execute_batch("BEGIN DEFERRED")?;
        let out = func(self);
        if out.is_err() {
            self.conn.execute_batch("ROLLBACK")?;
        } else {
            self.conn.execute_batch("COMMIT")?;
        }
        out
    }

    #[cfg(feature = "test")]
    fn reset(&mut self) -> Result<()> {
        let new_storage = SqliteStorage::new_ephemeral()?;
        *self = new_storage;
        self.init()?;
        Ok(())
    }

    fn trim(&mut self) -> Result<()> {
        // Remove non-canon blocks.
        self.conn
            .execute(r"DELETE FROM blocks WHERE blocks.canon_height IS NULL", [])?;

        // Remove hanging transactions
        self.conn.execute(
            r"
            DELETE FROM transaction_blocks
            WHERE id IN (
                SELECT tb.id FROM transaction_blocks tb
                LEFT JOIN blocks b ON b.id = tb.block_id WHERE b.id IS NULL
            );
        ",
            [],
        )?;

        self.conn.execute(
            r"
            DELETE FROM transactions
            WHERE id IN (
                SELECT t.id FROM transactions t
                LEFT JOIN transaction_blocks tb ON tb.transaction_id = t.id WHERE tb.id IS NULL
            );
        ",
            [],
        )?;

        // Compact the storage file.
        self.conn.execute("VACUUM", [])?;

        Ok(())
    }
}
