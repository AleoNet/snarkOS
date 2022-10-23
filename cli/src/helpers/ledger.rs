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

use snarkvm::prelude::{
    Address,
    Block,
    BlockMemory,
    Identifier,
    Network,
    PrivateKey,
    Program,
    ProgramID,
    ProgramMemory,
    ProgramStore,
    RecordsFilter,
    Transaction,
    Value,
    ViewKey,
    Zero,
    VM,
};

use anyhow::{bail, ensure, Result};
use core::str::FromStr;
use parking_lot::RwLock;
use std::{convert::TryFrom, sync::Arc};
use warp::{reply, Filter, Rejection};

pub(crate) type InternalStorage<N> = ProgramMemory<N>;
pub(crate) type InternalLedger<N> = snarkvm::prelude::Ledger<N, BlockMemory<N>, InternalStorage<N>>;
pub(crate) type InternalServer<N> = snarkvm::prelude::Server<N, BlockMemory<N>, InternalStorage<N>>;

#[allow(dead_code)]
pub struct Ledger<N: Network> {
    /// The internal ledger.
    pub ledger: Arc<RwLock<InternalLedger<N>>>,
    /// The runtime.
    runtime: tokio::runtime::Runtime,
    /// The server.
    server: InternalServer<N>,
    /// The account private key.
    private_key: PrivateKey<N>,
    /// The account view key.
    view_key: ViewKey<N>,
    /// The account address.
    address: Address<N>,
}

impl<N: Network> Ledger<N> {
    /// Initializes a new instance of the ledger.
    pub fn load(private_key: &PrivateKey<N>) -> Result<Arc<Self>> {
        // Derive the view key and address.
        let view_key = ViewKey::try_from(private_key)?;
        let address = Address::try_from(&view_key)?;

        // Initialize an RNG.
        let rng = &mut ::rand::thread_rng();
        // Initialize the store.
        let store = ProgramStore::<_, InternalStorage<_>>::open(None)?;
        // Create a genesis block.
        let genesis = Block::genesis(&VM::new(store)?, private_key, rng)?;

        // Initialize the ledger.
        let ledger = Arc::new(RwLock::new(InternalLedger::new_with_genesis(&genesis, address, None)?));

        // Initialize the additional routes.
        let additional_routes = {
            // GET /testnet3/development/privateKey
            let get_development_private_key = warp::get()
                .and(warp::path!("testnet3" / "development" / "privateKey"))
                .and(snarkvm::rest::with(*private_key))
                .and_then(|private_key: PrivateKey<N>| async move {
                    Ok::<_, Rejection>(reply::json(&private_key.to_string()))
                });

            // GET /testnet3/development/viewKey
            let get_development_view_key = warp::get()
                .and(warp::path!("testnet3" / "development" / "viewKey"))
                .and(snarkvm::rest::with(view_key))
                .and_then(|view_key: ViewKey<N>| async move { Ok::<_, Rejection>(reply::json(&view_key.to_string())) });

            // GET /testnet3/development/address
            let get_development_address = warp::get()
                .and(warp::path!("testnet3" / "development" / "address"))
                .and(snarkvm::rest::with(address))
                .and_then(|address: Address<N>| async move { Ok::<_, Rejection>(reply::json(&address.to_string())) });

            get_development_private_key.or(get_development_view_key).or(get_development_address)
        };

        // Initialize a runtime.
        let runtime =
            tokio::runtime::Builder::new_multi_thread().enable_all().thread_stack_size(8 * 1024 * 1024).build()?;

        // Initialize the server.
        let ledger_clone = ledger.clone();
        let server = runtime.block_on(async move {
            // Start the server.
            InternalServer::<N>::start(ledger_clone, Some(additional_routes), Some(4180))
        })?;

        // Return the ledger.
        Ok(Arc::new(Self { ledger, runtime, server, private_key: *private_key, view_key, address }))
    }

    /// Returns the account address.
    pub const fn address(&self) -> &Address<N> {
        &self.address
    }
}

impl<N: Network> Ledger<N> {
    /// Adds the given transaction to the memory pool.
    pub fn add_to_memory_pool(&self, transaction: Transaction<N>) -> Result<()> {
        self.ledger.write().add_to_memory_pool(transaction)
    }

    /// Advances the ledger to the next block.
    pub fn advance_to_next_block(&self) -> Result<Block<N>> {
        // Initialize an RNG.
        let rng = &mut ::rand::thread_rng();
        // Propose the next block.
        let next_block = self.ledger.read().propose_next_block(&self.private_key, rng)?;
        // Add the next block to the ledger.
        if let Err(error) = self.ledger.write().add_next_block(&next_block) {
            // Log the error.
            eprintln!("{error}");
        }
        // Return the next block.
        Ok(next_block)
    }

    /// Creates a deploy transaction.
    pub fn create_deploy(&self, program: &Program<N>, additional_fee: u64) -> Result<Transaction<N>> {
        // Fetch the unspent record with the most gates.
        let record = self
            .ledger
            .read()
            .find_records(&self.view_key, RecordsFilter::Unspent)?
            .max_by(|(_, a), (_, b)| (**a.gates()).cmp(&**b.gates()));

        // Prepare the additional fee.
        let credits = match record {
            Some((_, record)) => record,
            None => bail!("The Aleo account has no records to spend."),
        };
        ensure!(***credits.gates() >= additional_fee, "The additional fee exceeds the record balance.");

        // Deploy.
        let transaction = Transaction::deploy(
            self.ledger.read().vm(),
            &self.private_key,
            program,
            (credits, additional_fee),
            &mut rand::thread_rng(),
        )?;
        // Verify.
        assert!(self.ledger.read().vm().verify(&transaction));
        // Return the transaction.
        Ok(transaction)
    }

    /// Creates a transfer transaction.
    pub fn create_transfer(&self, to: &Address<N>, amount: u64) -> Result<Transaction<N>> {
        // Fetch the unspent record with the least gates.
        let record = self
            .ledger
            .read()
            .find_records(&self.view_key, RecordsFilter::Unspent)?
            .filter(|(_, record)| !record.gates().is_zero())
            .min_by(|(_, a), (_, b)| (**a.gates()).cmp(&**b.gates()));

        // Prepare the record.
        let record = match record {
            Some((_, record)) => record,
            None => bail!("The Aleo account has no records to spend."),
        };

        // Create a new transaction.
        Transaction::execute(
            self.ledger.read().vm(),
            &self.private_key,
            &ProgramID::from_str("credits.aleo")?,
            Identifier::from_str("transfer")?,
            &[Value::Record(record), Value::from_str(&format!("{to}"))?, Value::from_str(&format!("{amount}u64"))?],
            None,
            &mut rand::thread_rng(),
        )
    }
}
