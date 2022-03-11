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

#[cfg(feature = "postgres-tls")]
use std::fs;

#[cfg(feature = "postgres-tls")]
use native_tls::{Certificate, TlsConnector};
#[cfg(feature = "postgres-tls")]
use postgres_native_tls::MakeTlsConnector;
use time::OffsetDateTime;
#[cfg(not(feature = "postgres-tls"))]
use tokio_postgres::NoTls;
use tokio_postgres::{types::Type, Client, Error};
use tracing::*;

use crate::crawler::{Crawler, Opts};

/// Connects to a PostgreSQL database and creates the needed tables if they don't exist yet.
pub async fn initialize_storage(opts: &Opts) -> Result<Client, anyhow::Error> {
    // Prepare the connection config.
    let config = format!(
        "host={} port={} user={} password={} dbname={}",
        opts.postgres_host, opts.postgres_port, opts.postgres_user, opts.postgres_pass, opts.postgres_dbname
    );

    // Connect to the PostgreSQL database.
    #[cfg(feature = "postgres-tls")]
    let (client, connection) = {
        let cert = fs::read(&opts.postgres_cert_path)?;
        let cert = Certificate::from_pem(&cert)?;
        let connector = TlsConnector::builder().add_root_certificate(cert).build()?;
        let connector = MakeTlsConnector::new(connector);

        tokio_postgres::connect(&config, connector).await?
    };

    #[cfg(not(feature = "postgres-tls"))]
    let (client, connection) = tokio_postgres::connect(&config, NoTls).await?;

    // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("Storage connection error: {}", e);
        }
    });

    if opts.postgres_clean {
        client
            .batch_execute("DROP TABLE IF EXISTS nodes; DROP TABLE IF EXISTS network;")
            .await?;
        debug!("Persistent storage was cleaned");
    }

    client
        .batch_execute(
            "
        CREATE TABLE IF NOT EXISTS nodes (
            ip              INET NOT NULL,
            port            INTEGER NOT NULL,
            timestamp       TIMESTAMP WITH TIME ZONE,
            type            SMALLINT,
            version         INTEGER,
            state           SMALLINT,
            height          INTEGER,
            handshake_ms    INTEGER
        );

        CREATE TABLE IF NOT EXISTS network (
            timestamp      TIMESTAMP WITH TIME ZONE NOT NULL,
            nodes          INTEGER NOT NULL,
            connections    INTEGER NOT NULL
        );
    ",
        )
        .await?;

    debug!("Persistent storage is ready");

    Ok(client)
}

impl Crawler {
    pub async fn write_crawling_data(&self) -> Result<(), Error> {
        if let Some(ref storage) = self.storage {
            let nodes = self.known_network.nodes();

            let mut storage = storage.lock().await;
            let transaction = storage.transaction().await?;

            let per_node_stmt = transaction
                .prepare_typed("INSERT INTO nodes VALUES ($1, $2, $3, $4, $5, $6, $7, $8);", &[
                    Type::INET,
                    Type::INT4,
                    Type::TIMESTAMPTZ,
                    Type::INT2,
                    Type::INT4,
                    Type::INT2,
                    Type::INT8,
                    Type::INT4,
                ])
                .await?;

            for (addr, meta) in nodes.into_iter().filter(|(_, meta)| meta.timestamp.is_some()) {
                transaction
                    .execute(&per_node_stmt, &[
                        &addr.ip(),
                        &(addr.port() as i32),
                        &meta.timestamp.unwrap(),
                        &meta.state.as_ref().map(|s| s.node_type as i16),
                        &meta.state.as_ref().map(|s| s.version as i32),
                        &meta.state.as_ref().map(|s| s.state as i16),
                        &meta.state.as_ref().map(|s| s.height as i64),
                        &meta.handshake_time.map(|t| t.whole_milliseconds() as i32),
                    ])
                    .await?;
            }

            let network_stmt = transaction
                .prepare_typed("INSERT INTO network VALUES ($1, $2, $3);", &[
                    Type::TIMESTAMPTZ,
                    Type::INT4,
                    Type::INT4,
                ])
                .await?;

            transaction
                .execute(&network_stmt, &[
                    &OffsetDateTime::now_utc(),
                    &(self.known_network.num_nodes() as i32),
                    &(self.known_network.num_connections() as i32),
                ])
                .await?;

            transaction.commit().await?;
        }

        Ok(())
    }
}
