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

use axum::{routing::get, Router};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::services::{ServeDir, ServeFile};

const SERVER_URL: &str = "127.0.0.1:6060";

async fn start_server() {
    // Serve the 'assets/' directory.
    let serve_dir = ServeDir::new("assets").fallback(ServeFile::new("assets/index.html"));

    // Initialize the routes.
    let router = Router::new().route("/", get(|| async { "Hello, World!" })).fallback_service(serve_dir);

    // Run the server.
    println!("Starting server at '{SERVER_URL}'...");
    let rest_addr: SocketAddr = SERVER_URL.parse().unwrap();
    let rest_listener = TcpListener::bind(rest_addr).await.unwrap();
    axum::serve(rest_listener, router.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
}

#[tokio::main]
async fn main() {
    tokio::spawn(async move { start_server().await });
    open::that(format!("http://{SERVER_URL}/assets/index.html")).expect("Failed to open website");
    // Note: Do not move this.
    std::future::pending::<()>().await;
}
