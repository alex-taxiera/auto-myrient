use ferris_says::say;
use std::io::{stdout, BufWriter};

pub fn hello_world() {
  let stdout = stdout();
    let message = String::from("Hello fellow Rustaceans!");
    let width = message.chars().count();

    let mut writer = BufWriter::new(stdout.lock());
    say(&message, width, &mut writer).unwrap();
}
