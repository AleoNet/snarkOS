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

use std::collections::HashSet;
#[cfg(feature = "postgres-tls")]
use std::{fs, path::PathBuf};

use clap::Parser;
#[cfg(feature = "postgres-tls")]
use native_tls::{Certificate, TlsConnector};
#[cfg(feature = "postgres-tls")]
use postgres_native_tls::MakeTlsConnector;
use time::OffsetDateTime;
#[cfg(not(feature = "postgres-tls"))]
use tokio_postgres::NoTls;
use tokio_postgres::{types::Type, Client, Error};
use tracing::*;

use crate::{
    connection::Connection,
    crawler::{Crawler, Opts},
    metrics::NetworkMetrics,
};

#[derive(Debug, Parser)]
pub struct PostgresOpts {
    /// The hostname of the postgres instance (defaults to "localhost").
    #[clap(long = "postgres-host", default_value = "localhost", action)]
    pub host: String,
    /// The port of the postgres instance (defaults to 5432).
    #[clap(long = "postgres-port", default_value = "5432", action)]
    pub port: u16,
    /// The user of the postgres instance (defaults to "postgres").
    #[clap(long = "postgres-user", default_value = "postgres", action)]
    pub user: String,
    /// The password for the postgres instance (defaults to nothing).
    #[clap(long = "postgres-pass", default_value = "", action)]
    pub pass: String,
    /// The hostname of the postgres instance (defaults to "postgres").
    #[clap(long = "postgres-dbname", default_value = "postgres", action)]
    pub dbname: String,
    /// If set to `true`, re-creates the crawler's database tables.
    #[clap(long = "postgres-clean", action)]
    pub clean: bool,
    /// The path to a certificate file to be used for a TLS connection with the postgres instance.
    #[cfg(feature = "postgres-tls")]
    #[clap(long = "postgres-cert-path", action)]
    pub cert_path: PathBuf,
}

/// Connects to a PostgreSQL database and creates the needed tables if they don't exist yet.
pub async fn initialize_storage(opts: &Opts) -> Result<Client, anyhow::Error> {
    // Prepare the connection config.
    let config = format!(
        "host={} port={} user={} password={} dbname={}",
        opts.postgres.host, opts.postgres.port, opts.postgres.user, opts.postgres.pass, opts.postgres.dbname
    );

    // Connect to the PostgreSQL database.
    #[cfg(feature = "postgres-tls")]
    let (client, connection) = {
        let cert = fs::read(&opts.postgres.cert_path)?;
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

    if opts.postgres.clean {
        client
            .batch_execute(
                "
                DROP TABLE IF EXISTS node_states;
                DROP TABLE IF EXISTS node_centrality;
                DROP TABLE IF EXISTS network;
                DROP TABLE IF EXISTS connections;",
            )
            .await?;
        debug!("Persistent storage was cleaned");
    }

    client
        .batch_execute(
            "
        CREATE TABLE IF NOT EXISTS node_states (
            ip              INET NOT NULL,
            port            INTEGER NOT NULL,
            timestamp       TIMESTAMP WITH TIME ZONE,
            type            SMALLINT,
            version         INTEGER,
            state           SMALLINT,
            height          INTEGER,
            handshake_ms    INTEGER,
            UNIQUE          (ip, port, timestamp)
        );
        CREATE INDEX IF NOT EXISTS node_states_idx ON node_states (ip, port, timestamp);

        CREATE TABLE IF NOT EXISTS node_centrality (
            ip                  INET NOT NULL,
            port                INTEGER NOT NULL,
            timestamp           TIMESTAMP WITH TIME ZONE NOT NULL,
            connection_count    SMALLINT,
            prestige_score      REAL,
            fiedler_value       REAL
        );

        CREATE TABLE IF NOT EXISTS network (
            timestamp        TIMESTAMP WITH TIME ZONE NOT NULL,
            nodes            INTEGER NOT NULL,
            connections      INTEGER NOT NULL,
            density          REAL,
            fiedler_value    REAL,
            dcd              REAL
        );

        CREATE TABLE IF NOT EXISTS connections (
            timestamp    TIMESTAMP WITH TIME ZONE NOT NULL,
            ip1          INET NOT NULL,
            port1        INTEGER NOT NULL,
            ip2          INET NOT NULL,
            port2        INTEGER NOT NULL
        );
    ",
        )
        .await?;

    debug!("Persistent storage is ready");

    Ok(client)
}

impl Crawler {
    pub async fn write_crawling_data(&self, connections: HashSet<Connection>, metrics: Option<NetworkMetrics>) -> Result<(), Error> {
        let metrics = if let Some(metrics) = metrics {
            metrics
        } else {
            return Ok(());
        };

        if let Some(ref storage) = self.storage {
            // Procure a timestamp for network and connection details.
            let timestamp = OffsetDateTime::now_utc();

            let mut storage = storage.lock().await;
            let transaction = storage.transaction().await?;

            let node_state_stmt = transaction
                .prepare_typed(
                    "INSERT INTO node_states VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    ON CONFLICT (ip, port, timestamp) DO NOTHING;",
                    &[
                        Type::INET,
                        Type::INT4,
                        Type::TIMESTAMPTZ,
                        Type::INT2,
                        Type::INT4,
                        Type::INT2,
                        Type::INT8,
                        Type::INT4,
                    ],
                )
                .await?;

            let node_centrality_stmt = transaction
                .prepare_typed("INSERT INTO node_centrality VALUES ($1, $2, $3, $4, $5, $6);", &[
                    Type::INET,
                    Type::INT4,
                    Type::TIMESTAMPTZ,
                    Type::INT4,
                    Type::FLOAT4,
                    Type::FLOAT4,
                ])
                .await?;

            for (addr, meta, nc) in metrics.per_node {
                transaction
                    .execute(&node_state_stmt, &[
                        &addr.ip(),
                        &(addr.port() as i32),
                        &meta.timestamp,
                        &meta.state.as_ref().map(|s| s.node_type as i16),
                        &meta.state.as_ref().map(|s| s.version as i32),
                        &meta.state.as_ref().map(|s| s.status as i16),
                        &meta.state.as_ref().map(|s| s.height as i64),
                        &meta.handshake_time.map(|t| t.whole_milliseconds() as i32),
                    ])
                    .await?;

                transaction
                    .execute(&node_centrality_stmt, &[
                        &addr.ip(),
                        &(addr.port() as i32),
                        &timestamp,
                        &(nc.degree_centrality as i32),
                        &(nc.eigenvector_centrality as f32),
                        &(nc.fiedler_value as f32),
                    ])
                    .await?;
            }

            let network_stmt = transaction
                .prepare_typed("INSERT INTO network VALUES ($1, $2, $3, $4, $5, $6);", &[
                    Type::TIMESTAMPTZ,
                    Type::INT4,
                    Type::INT4,
                    Type::FLOAT4,
                    Type::FLOAT4,
                    Type::INT4,
                ])
                .await?;

            transaction
                .execute(&network_stmt, &[
                    &timestamp,
                    &(metrics.node_count as i32),
                    &(metrics.connection_count as i32),
                    &(metrics.density as f32),
                    &(metrics.algebraic_connectivity as f32),
                    &(metrics.degree_centrality_delta as i32),
                ])
                .await?;

            let connections_stmt = transaction
                .prepare_typed("INSERT INTO connections VALUES ($1, $2, $3, $4, $5);", &[
                    Type::TIMESTAMPTZ,
                    Type::INET,
                    Type::INT4,
                    Type::INET,
                    Type::INT4,
                ])
                .await?;

            for conn in connections {
                transaction
                    .execute(&connections_stmt, &[
                        &timestamp,
                        &conn.source.ip(),
                        &(conn.source.port() as i32),
                        &conn.target.ip(),
                        &(conn.target.port() as i32),
                    ])
                    .await?;
            }

            transaction.commit().await?;
        }

        Ok(())
    }
}
