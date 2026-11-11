//! Measures the letter-to-sound rules against the dictionary, for tuning them. For each word of standard input that
//! the dictionary has, it checks whether the rules alone give the same sounds (ignoring stress), and whether they
//! stress the same vowel. `-v` also lists the words whose sounds differ.
//! `cargo run -p vocal-synth --example rules -- [-v] < words.txt`
use std::io::{BufRead, stdin};
use vocal_synth::{Dictionary, ENGLISH, Speaker};

const VOWELS: [&str; 20] = ["aɪ", "aʊ", "eɪ", "oʊ", "ɔɪ", "ɑ", "æ", "ʌ", "ə", "ɔ", "ɛ", "ɝ", "ɚ", "ɪ", "i", "u", "ʊ", "a", "e", "o"];

/// The number of vowels in IPA text.
fn vowels(mut text: &str) -> usize {
    let mut count = 0;
    while let Some(c) = text.chars().next() {
        match VOWELS.iter().find(|v| text.starts_with(*v)) {
            Some(v) => {
                count += 1;
                text = &text[v.len()..];
            }
            None => text = &text[c.len_utf8()..],
        }
    }
    count
}

/// Which vowel carries the main stress, counted from 0.
fn stressed(transcript: &str) -> Option<usize> {
    transcript.split_once('ˈ').map(|(before, _)| vowels(before))
}

fn main() {
    let verbose = std::env::args().any(|a| a == "-v");
    let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../webapp/root_files/vocal-synth-en-us.dict"))
        .expect("the dictionary file");
    let dictionary = Dictionary::parse(&bytes, &ENGLISH).expect("a valid dictionary");
    let mut speaker = Speaker::new(16_000.0);
    let plain = |s: &str| s.chars().filter(|c| !matches!(c, 'ˈ' | 'ˌ')).collect::<String>();
    let (mut total, mut same, mut long, mut same_stress) = (0, 0, 0, 0);
    for word in stdin().lock().lines().map_while(Result::ok) {
        let word = word.trim().to_lowercase();
        if word.is_empty() || !dictionary.contains(&word) || speaker.speak_text(&word, Some(&dictionary)).is_err() {
            continue;
        }
        let known = speaker.transcript().to_string();
        speaker.speak_text(&word, None).expect("the rules read any word");
        let ruled = speaker.transcript().to_string();
        total += 1;
        if plain(&known) == plain(&ruled) {
            same += 1;
        } else if verbose {
            println!("{word:20} rules {ruled:20} dictionary {known}");
        }
        if vowels(&known) >= 2 {
            long += 1;
            if stressed(&known) == stressed(&ruled) {
                same_stress += 1;
            }
        }
    }
    let share = |a: usize, b: usize| 100.0 * a as f64 / b.max(1) as f64;
    println!("Sounds: {same} of {total} words ({:.1}%) match the dictionary", share(same, total));
    println!("Stress: {same_stress} of {long} words of two or more syllables ({:.1}%) match", share(same_stress, long));
}
