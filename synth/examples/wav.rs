//! Writes a WAV file of English text, for listening to changes outside the browser:
//! `cargo run -p vocal-synth --example wav -- out.wav "Text to say." [man|woman|child]`
use vocal_synth::{Dictionary, ENGLISH, Speaker, param};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (path, text) = (args.get(1).expect("an output path"), args.get(2).expect("text to say"));
    let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../webapp/root_files/vocal-synth-en-us.dict"))
        .expect("the dictionary file");
    let dictionary = Dictionary::parse(&bytes, &ENGLISH).expect("a valid dictionary");
    let rate: u32 = 44_100;
    let mut speaker = Speaker::new(rate as f32);
    // The voice presets from webapp/wc/voice-controls.js.
    let (pitch, tract, rd, breath) = match args.get(3).map(String::as_str) {
        Some("woman") => (210.0, 14.8, 1.4, 0.2),
        Some("child") => (290.0, 12.5, 1.3, 0.15),
        _ => (110.0, 17.5, 1.0, 0.1),
    };
    for (id, value) in [(param::PITCH, pitch), (param::TRACT_CM, tract), (param::RD, rd), (param::BREATH, breath)] {
        speaker.set(id, value);
    }
    speaker.speak_text(text, Some(&dictionary)).expect("text that fits in one render");
    let mut samples = vec![0.0; speaker.length() as usize];
    speaker.render(&mut samples);
    let mut wav = Vec::with_capacity(44 + samples.len() * 2);
    let data = (samples.len() * 2) as u32;
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    for v in [1_u16, 1] {
        wav.extend_from_slice(&v.to_le_bytes());
    }
    for v in [rate, rate * 2] {
        wav.extend_from_slice(&v.to_le_bytes());
    }
    for v in [2_u16, 16] {
        wav.extend_from_slice(&v.to_le_bytes());
    }
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data.to_le_bytes());
    for s in samples {
        wav.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    std::fs::write(path, wav).expect("a writable path");
    println!("{path}: {}", speaker.transcript());
}
