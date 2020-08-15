use colored::*;

pub fn render_init() -> String {
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
    .white()
    .bold();
    let aleo_welcome =
        format!("Welcome to Aleo! We thank you for supporting privacy and running a network node.\n\n").bold();
    let aleo_start = format!("{}{}", aleo_ascii, aleo_welcome);

    format!("{}\n", aleo_start)
}
