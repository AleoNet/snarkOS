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

use axum::Router;
use std::net::SocketAddr;

const SERVER_URL: &str = "127.0.0.1:6000";

async fn start_server() {
    // Initialize the routes.
    let router = Router::new().nest("/static", axum_static::static_router("/"));

    // Run the server.
    println!("Starting server at '{SERVER_URL}'...");
    axum::Server::bind(&SERVER_URL.parse().unwrap())
        .serve(router.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

#[tokio::main]
async fn main() {
    tokio::spawn(|| async { start_server().await });
    open::that(&format!("http://{SERVER_URL}"));
}
