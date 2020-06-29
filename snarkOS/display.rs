use colored::*;

pub fn render_init(miner_address: &str) -> String {
    let aleo_ascii = format!(
        r#"

         ╦╬╬╬╬╦
        ╬╬╬╬╬╬╠╬                 ▄▄▄▄▄▄     ▄▄▄▄       ▄▄▄▄▄▄▄▄▄▄   ▄▓▓▓▓▓▓▓▄
       ╬╬╬╬╬╬╬╬╬╬               ▓▓▓▓▓▓▓▓▌  ▐▓▓▓▓      ▐▓▓▓▓▓▓▓▓▓▌ ▄▓▓▓▓▓▓▓▓▓▓▓
      ╬╬╬╬╬╬╬╬╬╬╬╬             ▓▓▓▓  ▓▓▓█  ▐▓▓▓▓      ▐▓▓▓▓       ▓▓▓▓▌   ▓▓▓▓▌
     ╬╬╬╬╬╬╜╙╬╬╬╬╬╬           ▐▓▓▓▓  ▓▓▓▓▌ ▐▓▓▓▓      ▐▓▓▓▓▓▓▓▓▓  ▓▓▓▓▌   ▓▓▓▓▌
    ╬╬╬╬╬╬    ╬╬╬╬╬╬          ▓▓▓▓▓▓▓▓▓▓▓█ ▐▓▓▓▓      ▐▓▓▓▌       ▓▓▓▓▌   ▓▓▓▓▌
   ╬╬╬╬╬╬      ╬╬╬╬╬╬         ▓▓▓▓    ▓▓▓▓ ▐▓▓▓▓▓▓▓▓▓ ▐▓▓▓▓▓▓▓▓▓▌ ▀▓▓▓▓▓▓▓▓▓▓▓
  ╬╬╬╬╬╬        ╬╬╬╬╬╬        ▀▀▀▀    ▀▀▀▀  ▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀   ▀▀█████▀▀
   ╙╙╙╙          ╙╙╙╙

"#
    )
    .cyan()
    .bold();
    let aleo_welcome =
        format!("Welcome to Aleo! We thank you for supporting privacy and running a network node.\n\n").bold();
    let aleo_start = format!("{}{}", aleo_ascii, aleo_welcome);

    let aleo_address = miner_address.bold();
    let aleo_miner_address = format!("Your Aleo miner address is {}.", aleo_address);

    format!("{}{}\n", aleo_start, aleo_miner_address)
}
