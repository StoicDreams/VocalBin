//! English intonation: the pitch of each vowel, from the shape of its sentence.
//!
//! - **Declination:** the voice drifts down across a sentence and resets at the next one.
//! - **Accents:** the stressed syllable of each content word is raised, a little less each time (downstep). Function
//!   words such as "the" and "of" are not accented.
//! - **Nucleus:** the last accent of a phrase carries its tune. A statement, a question starting with a question word
//!   ("where", "how"), and an exclamation fall from it. A yes-no question falls low on it and rises to the end. A
//!   phrase that continues after a comma rises a little at its end, so the listener hears there is more.
//!
//! Each vowel gets a start and end pitch, so the nucleus can rise and fall within one syllable. The numbers are
//! fractions of the voice's pitch, before the intonation setting scales them, and are starting points to tune by ear.

use super::reader::{Said, Spoken};

const DECLINATION: f32 = 0.10;
const FIRST_ACCENT: f32 = 0.16;
const DOWNSTEP: f32 = 0.03;
const LOWEST_ACCENT: f32 = 0.08;
const SECONDARY: f32 = 0.03;
const UNSTRESSED: f32 = -0.02;
const FINAL_LOW: f32 = -0.16;
const QUESTION_HIGH: f32 = 0.32;
const CONTINUATION_RISE: f32 = 0.10;
/// Exclamations use a wider range.
const EXCLAMATION: f32 = 1.4;

const QUESTION_WORDS: [&str; 9] = ["who", "what", "when", "where", "why", "which", "whose", "whom", "how"];

#[derive(Clone, Copy, PartialEq)]
enum Tune {
    Fall,
    Rise,
    Continue,
}

/// One vowel in a sentence: which word and sound it is.
#[derive(Clone, Copy)]
struct Vowel {
    word: usize,
    sound: usize,
    stress: u8,
    function: bool,
}

/// Sets the tone of every vowel in `spoken`.
pub fn apply(spoken: &mut [Spoken]) {
    let mut start = 0;
    while start < spoken.len() {
        let end = spoken[start..]
            .iter()
            .position(|s| matches!(s, Spoken::Break { mark: '.' | '!' | '?', .. }))
            .map_or(spoken.len(), |i| start + i);
        let mark = match spoken.get(end) {
            Some(Spoken::Break { mark, .. }) => *mark,
            _ => '.',
        };
        sentence(&mut spoken[start..end], mark);
        start = end + 1;
    }
}

fn sentence(spoken: &mut [Spoken], mark: char) {
    let first_word = spoken.iter().find_map(|s| match s {
        Spoken::Word(w) => Some(w.text.clone()),
        _ => None,
    });
    let range = if mark == '!' { EXCLAMATION } else { 1.0 };
    let final_tune = match mark {
        '?' if !first_word.as_deref().is_some_and(|w| QUESTION_WORDS.contains(&w)) => Tune::Rise,
        _ => Tune::Fall,
    };

    let total = count_vowels(spoken);
    let mut seen = 0;
    let mut start = 0;
    while start <= spoken.len() {
        let end = spoken[start..].iter().position(|s| matches!(s, Spoken::Break { .. })).map_or(spoken.len(), |i| start + i);
        let tune = if end == spoken.len() { final_tune } else { Tune::Continue };
        seen += phrase(&mut spoken[start..end], tune, range, seen, total);
        start = end + 1;
    }
}

fn count_vowels(spoken: &[Spoken]) -> usize {
    spoken
        .iter()
        .map(|s| match s {
            Spoken::Word(w) => w.sounds.iter().filter(|(p, _)| p.is_vowel()).count(),
            _ => 0,
        })
        .sum()
}

/// Sets one phrase's tones. `before` is how many of the sentence's `total` vowels came before it. Returns how many
/// vowels it has.
fn phrase(spoken: &mut [Spoken], tune: Tune, range: f32, before: usize, total: usize) -> usize {
    let mut vowels = Vec::new();
    for (word, item) in spoken.iter().enumerate() {
        if let Spoken::Word(Said { sounds, function, .. }) = item {
            for (sound, &(p, stress)) in sounds.iter().enumerate() {
                if p.is_vowel() {
                    vowels.push(Vowel { word, sound, stress, function: *function });
                }
            }
        }
    }
    let Some(last) = vowels.len().checked_sub(1) else { return 0 };

    // Accents: content words' primary stress, or any stress, or the last vowel.
    let mut accents: Vec<usize> = (0..vowels.len()).filter(|&i| vowels[i].stress == 1 && !vowels[i].function).collect();
    if accents.is_empty() {
        accents = (0..vowels.len()).filter(|&i| vowels[i].stress > 0).collect();
    }
    if accents.is_empty() {
        accents.push(last);
    }
    let nucleus = *accents.last().expect("there is an accent");

    let base = |i: usize| -DECLINATION * (before + i) as f32 / total.saturating_sub(1).max(1) as f32;
    let mut tones = vec![(0.0, 0.0); vowels.len()];
    for (i, v) in vowels.iter().enumerate().take(nucleus) {
        let level = if let Some(k) = accents.iter().position(|&a| a == i) {
            (FIRST_ACCENT - DOWNSTEP * k as f32).max(LOWEST_ACCENT) * range
        } else if v.stress == 2 && !v.function {
            SECONDARY
        } else {
            UNSTRESSED
        };
        tones[i] = (base(i) + level, base(i) + level);
    }

    let peak = if accents.len() == 1 { FIRST_ACCENT } else { LOWEST_ACCENT + DOWNSTEP } * range;
    let low = FINAL_LOW * range;
    let tail = last - nucleus;
    match tune {
        Tune::Fall => {
            let b = base(nucleus);
            tones[nucleus] = if tail == 0 { (b + peak, b + low) } else { (b + peak, b + peak * 0.5) };
            for tone in &mut tones[nucleus + 1..] {
                *tone = (b + low * 0.8, b + low);
            }
        }
        Tune::Rise => {
            let b = base(nucleus);
            tones[nucleus] = if tail == 0 { (b - 0.04, QUESTION_HIGH) } else { (b - 0.04, b - 0.02) };
            let level = |i: usize| b - 0.02 + (QUESTION_HIGH - b + 0.02) * (i - nucleus) as f32 / tail.max(1) as f32;
            for (i, tone) in tones.iter_mut().enumerate().skip(nucleus + 1) {
                *tone = (level(i - 1), level(i));
            }
        }
        Tune::Continue => {
            let b = base(nucleus);
            tones[nucleus] = if tail == 0 { (b + peak * 0.6, b + CONTINUATION_RISE) } else { (b + peak * 0.8, b + peak * 0.4) };
            for (i, tone) in tones.iter_mut().enumerate().skip(nucleus + 1) {
                let rise = if i == last { CONTINUATION_RISE } else { 0.0 };
                *tone = (b + UNSTRESSED, b + rise);
            }
        }
    }

    for (v, tone) in vowels.iter().zip(tones) {
        if let Spoken::Word(said) = &mut spoken[v.word] {
            said.tones[v.sound] = Some(tone);
        }
    }
    vowels.len()
}

#[cfg(test)]
mod tests {
    use super::super::reader::{read, tests::dictionary};
    use crate::phonemes::Token;

    /// Each vowel's tone, in order.
    fn tones(text: &str) -> Vec<(f32, f32)> {
        read(text, Some(dictionary()))
            .unwrap()
            .tokens
            .iter()
            .filter_map(|t| match *t {
                Token::Phone { phoneme, tone, .. } if phoneme.is_vowel() => tone,
                _ => None,
            })
            .collect()
    }

    #[test]
    fn every_vowel_has_a_tone() {
        let reading = read("Well, how are you? Fine!", Some(dictionary())).unwrap();
        for token in &reading.tokens {
            if let Token::Phone { phoneme, tone, .. } = token {
                assert_eq!(tone.is_some(), phoneme.is_vowel(), "{}", phoneme.ipa);
            }
        }
    }

    #[test]
    fn statements_fall_at_the_end() {
        let t = tones("The dog ran home.");
        let last = t.last().unwrap();
        assert!(last.1 < -0.1 && last.1 < last.0, "{t:?}");
    }

    #[test]
    fn yes_no_questions_rise_and_question_word_questions_fall() {
        let yes_no = tones("Is it raining?");
        let wh = tones("Where is it raining?");
        assert!(yes_no.last().unwrap().1 > 0.2, "{yes_no:?}");
        assert!(wh.last().unwrap().1 < -0.1, "{wh:?}");
    }

    #[test]
    fn a_comma_rises_a_little() {
        let t = tones("When it rains, we stay home.");
        // "rains" ends the first phrase.
        assert!(t[2].1 > t[2].0 - 0.05 && t[2].1 > 0.0, "{t:?}");
    }

    #[test]
    fn content_words_are_accented_and_function_words_are_not() {
        let t = tones("the cat of the house");
        assert!(t[1].0 > t[0].0 + 0.1 && t[1].0 > t[2].0 + 0.1, "{t:?}");
    }

    #[test]
    fn accents_step_down() {
        let t = tones("Big red dogs run very fast today.");
        assert!(t[0].0 > t[1].0 && t[1].0 > t[2].0, "{t:?}");
    }
}
