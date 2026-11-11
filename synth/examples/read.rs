//! Prints how the English reader says each line of standard input, for checking pronunciation by eye:
//! `cargo run -p vocal-synth --example read < lines.txt`
use std::io::{BufRead, stdin};
use vocal_synth::{Dictionary, ENGLISH, Speaker};

fn main() {
    let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../webapp/root_files/vocal-synth-en-us.dict"))
        .expect("the dictionary file");
    let dictionary = Dictionary::parse(&bytes, &ENGLISH).expect("a valid dictionary");
    let mut speaker = Speaker::new(16_000.0);
    for line in stdin().lock().lines().map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }
        match speaker.speak_text(&line, Some(&dictionary)) {
            Ok(()) => println!("{line}\n    {}\n", speaker.transcript()),
            Err(error) => println!("{line}\n    error: {error:?}\n"),
        }
    }
}
