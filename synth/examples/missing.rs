//! Prints the words of standard input (one per line) that the English dictionary lacks, with how the reader says
//! them: `cargo run -p vocal-synth --example missing < words.txt`
use std::io::{BufRead, stdin};
use vocal_synth::{Dictionary, ENGLISH, Speaker};

fn main() {
    let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../webapp/root_files/vocal-synth-en-us.dict"))
        .expect("the dictionary file");
    let dictionary = Dictionary::parse(&bytes, &ENGLISH).expect("a valid dictionary");
    let mut speaker = Speaker::new(16_000.0);
    for word in stdin().lock().lines().map_while(Result::ok) {
        let word = word.trim().to_lowercase();
        if word.is_empty() || dictionary.contains(&word) {
            continue;
        }
        if speaker.speak_text(&word, Some(&dictionary)).is_ok() {
            println!("{word:24} {}", speaker.transcript());
        }
    }
}
