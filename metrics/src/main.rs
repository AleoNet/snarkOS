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

use snarkos_metrics::prometheus::{initialize, metrics_handler, CONNECTED_PEERS};

use futures_util::StreamExt;
use std::time::Duration;
use tokio::task;
use warp::{ws::WebSocket, Filter, Rejection, Reply};

async fn ws_client_connection(ws: WebSocket, id: String) {
    let (_client_ws_sender, mut client_ws_rcv) = ws.split();

    CONNECTED_PEERS.inc();
    println!("{} connected", id);

    while let Some(result) = client_ws_rcv.next().await {
        match result {
            Ok(msg) => println!("received message: {:?}", msg),
            Err(e) => {
                eprintln!("error receiving ws message for id: {}): {}", id.clone(), e);
                break;
            }
        };
    }

    println!("{} disconnected", id);
    CONNECTED_PEERS.dec();
}

pub async fn ws_handler(ws: warp::ws::Ws, id: String) -> Result<impl Reply, Rejection> {
    Ok(ws.on_upgrade(move |socket| ws_client_connection(socket, id)))
}

pub async fn connected_peers_handler() -> Result<impl Reply, Rejection> {
    CONNECTED_PEERS.inc();
    Ok("hello!")
}

pub async fn data_collector() {
    let mut collect_interval = tokio::time::interval(Duration::from_millis(1000));
    loop {
        collect_interval.tick().await;

        println!("testing - Incrementing connected_peers");
        CONNECTED_PEERS.inc();
    }
}

#[tokio::main]
async fn main() {
    initialize();

    let metrics_route = warp::path!("metrics").and_then(metrics_handler);
    let connected_peers_route = warp::path!("connected_peers").and_then(connected_peers_handler);
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::path::param())
        .and_then(ws_handler);

    task::spawn(data_collector());

    println!("Started on port 8080");
    warp::serve(metrics_route.or(connected_peers_route).or(ws_route))
        .run(([0, 0, 0, 0], 8080))
        .await;
}
