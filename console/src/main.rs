use snarkos_console::{node_console::ConsoleCli, CLI};

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let arguments = ConsoleCli::new();

    let response = ConsoleCli::make_request(ConsoleCli::parse(&arguments)?).await?;

    println!("{}", response);

    Ok(())
}
