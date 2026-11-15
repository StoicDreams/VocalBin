//! Stage 2: speaks phonemes, written in IPA or (for English) ARPAbet. Each phoneme becomes one or more timed steps of
//! [`Controls`], and the voice glides between them, so transitions come from the same spring-like motion as the
//! vowel lab's glides. Nothing here depends on the language: the sounds come from the language's table, and the
//! steps from each sound's [`Kind`] and [`Place`].
//!
//! Stage 3 reads ordinary text with the language's reader, which also sets each vowel's pitch (its intonation). IPA
//! and ARPAbet have no such pitch, so for them the speaker's own simple intonation applies: pitch drifts down across
//! a phrase, stressed vowels rise, and the phrase ends falling, or rising for `?`.
//!
//! Stage 4 makes sounds blend as they do in speech (coarticulation), from the sounds' kinds and places:
//!
//! - **Consonants lean toward their vowel:** a consonant's formants point part of the way from its own locus toward
//!   the vowel next to it, more for the lips (which leave the tongue free) than for the tongue tip. These are locus
//!   equations, measured in many languages: F2 at a consonant's release grows with the vowel's F2 along a line whose
//!   slope depends on the place. Velars also meet the vowel where it needs the tongue (the velar pinch).
//! - **Weak vowels fall short:** unstressed and reduced vowels don't reach their targets, and take on the color of
//!   the sounds around them (Lindblom's undershoot), more so when speaking fast.
//! - **Timing:** besides stress and phrase-final lengthening, a vowel is shorter before a voiceless consonant ("bit"
//!   against "bid"), vowels are shorter in longer words, and consonants are shorter in clusters, after Klatt's
//!   duration rules.
//! - **Variants:** a language can swap sounds for their variants in context first (`Language::allophones`), such as
//!   the American tap in "butter".

use crate::dictionary::Dictionary;
use crate::english::{ENGLISH, parse_arpabet};
use crate::language::{Language, Notation};
use crate::phonemes::{Kind, NEUTRAL, ParseError, Phoneme, Place, Token, burst};
use crate::voice::{Controls, Voice, param};

/// One phoneme or pause in the rendered audio, and where it came from in the text (in bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Item {
    pub start: u32,
    pub end: u32,
    pub offset: u32,
    pub len: u32,
}

#[derive(Clone, Copy, Debug)]
struct Step {
    samples: u32,
    controls: Controls,
}

const LEAD_IN_MS: f32 = 40.0;
const TAIL_MS: f32 = 150.0;
const COMMA_MS: f32 = 220.0;
const SENTENCE_MS: f32 = 420.0;
const FINAL_LENGTHENING: f32 = 1.3;
/// A vowel before a voiceless consonant, as in "bit" against "bid".
const BEFORE_VOICELESS: f32 = 0.85;
/// A vowel in a word of two or more syllables. Only text marks words, so IPA and ARPAbet don't use it.
const POLYSYLLABIC: f32 = 0.85;
/// A consonant next to another consonant.
const IN_CLUSTER: f32 = 0.8;
/// How far an unstressed vowel falls short of its target toward the sounds around it, and a reduced one such as
/// /ə/, at normal speed. Secondary stress falls short by a third of the unstressed amount.
const UNDERSHOOT: f32 = 0.3;
const REDUCED_UNDERSHOOT: f32 = 0.45;

/// How far a consonant's F2 moves from its locus toward the vowel next to it: the slope of its locus equation.
/// The lips leave the tongue free to take the vowel's shape, and the tongue tip holds its own place most.
fn locus_slope(place: Place) -> f32 {
    match place {
        Place::Labial | Place::Velar => 0.8,
        Place::Dental => 0.55,
        Place::Alveolar | Place::Postalveolar => 0.45,
        Place::Palatal => 0.3,
        _ => 1.0,
    }
}

pub struct Speaker {
    voice: Voice,
    language: &'static Language,
    rate: f32,
    intonation: f32,
    steps: Vec<Step>,
    items: Vec<Item>,
    next_step: usize,
    left: u32,
    length: u32,
    block: Vec<f32>,
    transcript: String,
}

impl Speaker {
    pub fn new(sample_rate: f32) -> Self {
        Speaker {
            voice: Voice::new(sample_rate),
            language: &ENGLISH,
            rate: 1.0,
            intonation: 1.0,
            steps: Vec::new(),
            items: Vec::new(),
            next_step: 0,
            left: 0,
            length: 0,
            block: Vec::new(),
            transcript: String::new(),
        }
    }

    /// Sets a voice setting, or the rate or intonation for the next text. Unknown numbers are ignored.
    pub fn set(&mut self, id: u32, value: f32) {
        if !value.is_finite() || self.voice.set(id, value) {
            return;
        }
        match id {
            param::RATE => self.rate = value.clamp(0.5, 2.0),
            param::INTONATION => self.intonation = value.clamp(0.0, 2.0),
            _ => {}
        }
    }

    /// The language IPA text is read in. English until another is set.
    pub fn language(&self) -> &'static Language {
        self.language
    }

    pub fn set_language(&mut self, language: &'static Language) {
        self.language = language;
    }

    /// Prepares `text` to be rendered from its start. Ordinary text is read without a dictionary, so every word comes
    /// from spelling rules; use `speak_text` to give it one. On an error, nothing changes.
    pub fn speak(&mut self, text: &str, notation: Notation) -> Result<(), ParseError> {
        let tokens = match notation {
            Notation::Ipa => self.language.parse_ipa(text)?,
            Notation::Arpabet => parse_arpabet(text)?,
            Notation::Text => return self.speak_text(text, None),
        };
        self.speak_tokens(&tokens);
        self.transcript.clear();
        Ok(())
    }

    /// Prepares ordinary text in the speaker's language, with its pronouncing dictionary if one is loaded. Afterwards
    /// `transcript` has what will be said, in IPA. On an error, nothing changes.
    pub fn speak_text(&mut self, text: &str, dictionary: Option<&Dictionary>) -> Result<(), ParseError> {
        let read = self.language.read_text.ok_or(ParseError { offset: 0, len: text.len() })?;
        let reading = read(text, dictionary)?;
        self.speak_tokens(&reading.tokens);
        self.transcript = reading.transcript;
        Ok(())
    }

    /// What the last ordinary text will sound like, in IPA, a word at a time. Empty for IPA or ARPAbet.
    pub fn transcript(&self) -> &str {
        &self.transcript
    }

    /// Prepares phonemes that are already read, such as from a dictionary, to be rendered from their start.
    pub fn speak_tokens(&mut self, tokens: &[Token]) {
        let mut tokens = tokens.to_vec();
        if let Some(allophones) = self.language.allophones {
            allophones(&mut tokens);
        }
        let (steps, items) = plan(&tokens, self.voice.sample_rate(), self.rate, self.intonation);
        self.length = steps.iter().map(|s| s.samples).sum();
        self.steps = steps;
        self.items = items;
        self.next_step = 0;
        self.left = 0;
        if let Some(first) = self.steps.first() {
            self.voice.set_controls(first.controls);
            self.voice.snap();
        }
    }

    /// The length of the text being spoken, in samples, including a short silence at each end.
    pub fn length(&self) -> u32 {
        self.length
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// The current frequency of formant `n` (0 to 4).
    pub fn formant(&self, n: usize) -> f32 {
        self.voice.formant(n)
    }

    /// The current pitch in Hz, before jitter, flutter, and vibrato.
    pub fn pitch(&self) -> f32 {
        self.voice.pitch_now()
    }

    /// Fills `out` with the next samples. After the end of the text, it is silent.
    pub fn render(&mut self, out: &mut [f32]) {
        for sample in out.iter_mut() {
            while self.left == 0 {
                match self.steps.get(self.next_step) {
                    Some(step) => {
                        self.voice.set_controls(step.controls);
                        self.left = step.samples;
                        self.next_step += 1;
                    }
                    None => {
                        let quiet = Controls { formants: self.voice.controls().formants, ..Controls::SILENT };
                        self.voice.set_controls(quiet);
                        self.left = u32::MAX;
                    }
                }
            }
            if self.left != u32::MAX {
                self.left -= 1;
            }
            *sample = self.voice.next_sample();
        }
    }

    /// Renders `frames` samples into a buffer the speaker owns, for callers that can't pass memory in.
    pub fn render_block(&mut self, frames: usize) -> &[f32] {
        let mut block = std::mem::take(&mut self.block);
        block.resize(frames, 0.0);
        self.render(&mut block);
        self.block = block;
        &self.block
    }
}

/// Each phone's formant target in its phrase: consonants lean toward the nearest vowel (the next one, or the one
/// before at a phrase end), and weak vowels fall short toward their neighbors' own targets.
fn targets(phones: &[Phone], rate: f32) -> Vec<[f32; 3]> {
    let vowel_near = |i: usize| {
        phones[i + 1..]
            .iter()
            .find(|p| p.phoneme.is_vowel())
            .map(|p| p.phoneme.formants)
            .or_else(|| phones[..i].iter().rev().find(|p| p.phoneme.is_vowel()).map(|p| p.phoneme.end))
    };
    let mut targets: Vec<[f32; 3]> = phones
        .iter()
        .enumerate()
        .map(|(i, phone)| {
            let p = phone.phoneme;
            let mut f = p.formants;
            if p.is_vowel() || p.kind == Kind::Aspirate {
                return f;
            }
            if let Some(v) = vowel_near(i) {
                // Approximants keep most of their own shape, which is what makes them heard, and /ɹ/ its low F3.
                let approximant = p.kind == Kind::Approximant;
                let slope = locus_slope(p.place) * if approximant { 0.3 } else { 1.0 };
                f[1] += slope * (v[1] - f[1]);
                if !approximant {
                    f[2] += 0.5 * slope * (v[2] - f[2]);
                }
            }
            f
        })
        .collect();
    let speed = rate.sqrt();
    for i in 0..phones.len() {
        let (p, stress) = (phones[i].phoneme, phones[i].stress);
        if p.kind != Kind::Vowel {
            continue;
        }
        let amount = if p.reduced {
            REDUCED_UNDERSHOOT
        } else {
            match stress {
                0 => UNDERSHOOT,
                2 => UNDERSHOOT / 3.0,
                _ => 0.0,
            }
        };
        // Toward a consonant's own locus, not where it leans toward this vowel: a short vowel runs out of time on
        // its way from one consonant's place to the next.
        let neighbor = |j: usize| {
            phones.get(j).map(|n| match (n.phoneme.is_vowel(), j < i) {
                (true, true) => n.phoneme.end,
                _ => n.phoneme.formants,
            })
        };
        let neighbors: Vec<[f32; 3]> = [i.checked_sub(1), Some(i + 1)].into_iter().flatten().filter_map(neighbor).collect();
        if amount == 0.0 || neighbors.is_empty() {
            continue;
        }
        let amount = (amount * speed).min(0.6);
        // F1 and F2 move toward the neighbors. F3 stays, which keeps /ɚ/ its color.
        for n in 0..2 {
            let context = neighbors.iter().map(|f| f[n]).sum::<f32>() / neighbors.len() as f32;
            targets[i][n] += amount * (context - targets[i][n]);
        }
    }
    targets
}

struct Phone {
    phoneme: &'static Phoneme,
    stress: u8,
    tone: Option<(f32, f32)>,
    offset: usize,
    len: usize,
}

/// Builds the steps for a whole text, and the item for each phoneme and pause.
fn plan(tokens: &[Token], sample_rate: f32, rate: f32, intonation: f32) -> (Vec<Step>, Vec<Item>) {
    let samples = |ms: f32| (ms.max(0.0) * sample_rate / 1000.0).round() as u32;
    let mut steps = Vec::new();
    let mut items = Vec::new();
    let mut time = 0_u32;

    let first_formants = tokens.iter().find_map(|t| match t {
        Token::Phone { phoneme, .. } => Some(phoneme.formants),
        _ => None,
    });
    let lead_in = Step { samples: samples(LEAD_IN_MS), controls: Controls { formants: first_formants.unwrap_or(NEUTRAL), ..Controls::SILENT } };
    time += lead_in.samples;
    steps.push(lead_in);

    // Split into phrases at each pause. A phrase keeps the mark that ends it.
    let mut start = 0;
    while start < tokens.len() {
        let end = tokens[start..].iter().position(|t| matches!(t, Token::Pause { .. })).map_or(tokens.len(), |i| start + i);
        let phones: Vec<Phone> = tokens[start..end]
            .iter()
            .filter_map(|t| match *t {
                Token::Phone { phoneme, stress, tone, offset, len } => Some(Phone { phoneme, stress, tone, offset, len }),
                _ => None,
            })
            .collect();
        let mark = match tokens.get(end) {
            Some(Token::Pause { mark, .. }) => Some(*mark),
            _ => None,
        };
        let mut phrase_steps = Vec::new();
        let ranges = speak_phrase(&phones, mark, rate, intonation, &samples, &mut phrase_steps);
        for (phone, (from, to)) in phones.iter().zip(ranges) {
            items.push(Item { start: time + from, end: time + to, offset: phone.offset as u32, len: phone.len as u32 });
        }
        time += phrase_steps.iter().map(|s| s.samples).sum::<u32>();
        steps.extend(phrase_steps);
        if let Some(Token::Pause { mark, offset }) = tokens.get(end) {
            let ms = if *mark == ',' { COMMA_MS } else { SENTENCE_MS };
            let last = steps.last().map_or(NEUTRAL, |s| s.controls.formants);
            let pause = Step { samples: samples(ms / rate.sqrt()), controls: Controls { formants: last, ..Controls::SILENT } };
            items.push(Item { start: time, end: time + pause.samples, offset: *offset as u32, len: 1 });
            time += pause.samples;
            steps.push(pause);
        }
        start = end + 1;
    }

    let last = steps.last().map_or(NEUTRAL, |s| s.controls.formants);
    steps.push(Step { samples: samples(TAIL_MS), controls: Controls { formants: last, ..Controls::SILENT } });
    (steps, items)
}

/// Builds one phrase's steps. Returns each phone's start and end, in samples from the phrase's start.
fn speak_phrase(
    phones: &[Phone],
    mark: Option<char>,
    rate: f32,
    intonation: f32,
    samples: &dyn Fn(f32) -> u32,
    steps: &mut Vec<Step>,
) -> Vec<(u32, u32)> {
    let vowels: Vec<usize> = (0..phones.len()).filter(|&i| phones[i].phoneme.is_vowel()).collect();
    let last_vowel = vowels.last().copied();

    // Pitch at the start and end of each phone. A language's intonation sets the vowels' tones, and a consonant takes
    // the pitch where the voice is. Without tones, the speaker's simple intonation applies.
    let mut pitch = vec![1.0_f32; phones.len()];
    let mut pitch_end = vec![1.0_f32; phones.len()];
    let tones = phones.iter().any(|p| p.tone.is_some());
    let mut current = 1.0;
    for i in 0..phones.len() {
        if tones {
            if let Some((start, end)) = phones[i].tone {
                pitch[i] = 1.0 + intonation * start;
                current = 1.0 + intonation * end;
            } else {
                pitch[i] = current;
            }
            pitch_end[i] = current;
            continue;
        }
        if let Some(n) = vowels.iter().position(|&v| v == i) {
            let progress = if vowels.len() > 1 { n as f32 / (vowels.len() - 1) as f32 } else { 0.0 };
            let declination = 1.0 - 0.10 * intonation * progress;
            let accent = intonation * match phones[i].stress {
                1 => 0.10,
                2 => 0.04,
                _ => -0.02,
            };
            current = if Some(i) == last_vowel {
                match mark {
                    Some('?') => 1.0 + 0.30 * intonation,
                    Some(',') => declination + 0.06 * intonation,
                    _ => declination - 0.12 * intonation,
                }
            } else {
                declination + accent
            };
        }
        pitch[i] = current;
        pitch_end[i] = current;
    }
    // Consonants before the first vowel take its pitch.
    if let Some(&first) = vowels.first() {
        let start = if tones { pitch[first] } else { 1.0 + 0.10 * intonation };
        for p in pitch.iter_mut().take(first) {
            *p = start;
        }
        for p in pitch_end.iter_mut().take(first) {
            *p = start;
        }
    }

    let next_vowel = |i: usize| phones[i + 1..].iter().find(|p| p.phoneme.is_vowel());
    let targets = targets(phones, rate);
    // The phones of each word, where the text marks words: they share a place in the text.
    let same_word = |a: &Phone, b: &Phone| a.offset == b.offset && a.len == b.len;
    let vowels_in_word = |i: usize| {
        let (mut from, mut to) = (i, i);
        while from > 0 && same_word(&phones[from - 1], &phones[i]) {
            from -= 1;
        }
        while to + 1 < phones.len() && same_word(&phones[to + 1], &phones[i]) {
            to += 1;
        }
        phones[from..=to].iter().filter(|p| p.phoneme.is_vowel()).count()
    };
    let is_consonant = |j: Option<usize>| j.and_then(|j| phones.get(j)).is_some_and(|n| !n.phoneme.is_vowel());
    let mut ranges = Vec::with_capacity(phones.len());
    let mut time = 0_u32;
    for (i, phone) in phones.iter().enumerate() {
        let p = phone.phoneme;
        let lengthen = if last_vowel.is_some_and(|v| i >= v) { FINAL_LENGTHENING } else { 1.0 };
        let next = phones.get(i + 1);
        let clustered = !p.is_vowel() && p.kind != Kind::Tap && (is_consonant(i.checked_sub(1)) || is_consonant(Some(i + 1)));
        let cluster = if clustered { IN_CLUSTER } else { 1.0 };
        let sonorant_ms = |ms: f32| ms * lengthen * cluster / rate;
        let closure_ms = |ms: f32| ms * cluster / rate.powf(0.7);
        // Where the formants head after a consonant: the next sound's start, or a neutral vowel.
        let next_formants = if next.is_some() { targets[i + 1] } else { NEUTRAL };
        let after_vowel = next_vowel(i).map_or(NEUTRAL, |n| n.phoneme.formants);
        let base = Controls { formants: targets[i], pitch_scale: pitch[i], ..Controls::SILENT };
        let from = time;
        let mut push = |ms: f32, controls: Controls, steps: &mut Vec<Step>| {
            let step = Step { samples: samples(ms).max(1), controls };
            time += step.samples;
            steps.push(step);
        };
        match p.kind {
            Kind::Vowel | Kind::Diphthong => {
                let stress = match phone.stress {
                    0 => 0.6,
                    2 => 0.8,
                    _ => 1.0,
                };
                let before_voiceless = next.is_some_and(|n| matches!(n.phoneme.kind, Kind::Plosive | Kind::Affricate | Kind::Fricative) && !n.phoneme.voiced);
                let timing = if before_voiceless { BEFORE_VOICELESS } else { 1.0 } * if vowels_in_word(i) > 1 { POLYSYLLABIC } else { 1.0 };
                let ms = sonorant_ms(p.ms * stress * timing);
                let glide_ms = match i.checked_sub(1).map(|j| phones[j].phoneme.kind) {
                    Some(Kind::Plosive | Kind::Affricate) => 45.0,
                    Some(Kind::Tap) => 30.0,
                    Some(Kind::Nasal) => 40.0,
                    Some(Kind::Fricative | Kind::Aspirate) => 50.0,
                    Some(Kind::Approximant) => 75.0,
                    Some(Kind::Vowel | Kind::Diphthong) => 90.0,
                    None => 40.0,
                };
                let vowel = Controls { glide_ms, voicing: 1.0, ..base };
                let end_pitch = pitch_end[i];
                if p.kind == Kind::Diphthong {
                    push(ms * 0.4, vowel, steps);
                    push(ms * 0.6, Controls { formants: p.end, glide_ms: ms * 0.6, pitch_scale: end_pitch, ..vowel }, steps);
                } else if end_pitch != pitch[i] {
                    // A rise or fall within the vowel.
                    push(ms * 0.45, vowel, steps);
                    push(ms * 0.55, Controls { pitch_scale: end_pitch, ..vowel }, steps);
                } else {
                    push(ms, vowel, steps);
                }
            }
            Kind::Approximant => push(sonorant_ms(p.ms), Controls { glide_ms: 50.0, voicing: 0.6, ..base }, steps),
            Kind::Nasal => push(sonorant_ms(p.ms), Controls { glide_ms: 30.0, voicing: 0.22, nasal_zero: p.nasal_zero, ..base }, steps),
            Kind::Aspirate => push(sonorant_ms(p.ms), Controls { formants: after_vowel, glide_ms: 25.0, aspiration: 1.0, ..base }, steps),
            Kind::Fricative => {
                let level = if p.voiced { 0.5 } else { 1.0 };
                let voicing = if p.voiced { 0.25 } else { 0.0 };
                push(sonorant_ms(p.ms), Controls { glide_ms: 45.0, voicing, frication: p.noise.map(|n| n * level), ..base }, steps);
            }
            // A quick dip in voicing with the tongue at its place, and no burst.
            Kind::Tap => push(closure_ms(p.ms), Controls { glide_ms: 15.0, voicing: 0.45, ..base }, steps),
            Kind::Plosive | Kind::Affricate => {
                let mut locus = targets[i];
                if p.place == Place::Velar {
                    // The tongue back meets the palate where the next vowel needs it: further forward before front
                    // vowels. F2 and F3 come together there (the velar pinch).
                    locus[1] = (after_vowel[1] + 150.0).clamp(1300.0, 2400.0);
                    locus[2] = locus[1] + 350.0;
                }
                let closure = Controls { formants: locus, glide_ms: 35.0, voicing: if p.voiced { 0.12 } else { 0.0 }, ..base };
                push(closure_ms(p.ms), closure, steps);
                let burst_ms = if p.place == Place::Velar { 15.0 } else { 10.0 };
                push(burst_ms, Controls { formants: locus, glide_ms: 35.0, frication: burst(p.place), ..base }, steps);
                if p.kind == Kind::Affricate {
                    let voicing = if p.voiced { 0.3 } else { 0.0 };
                    let level = if p.voiced { 0.6 } else { 1.0 };
                    let hiss = Controls { formants: locus, glide_ms: 45.0, voicing, frication: p.noise.map(|n| n * level), ..base };
                    push(sonorant_ms(p.release_ms), hiss, steps);
                } else {
                    let vot_ms = p.release_ms / rate.sqrt();
                    let (aspiration, voicing) = if p.voiced { (0.2, 0.3) } else { (0.7, 0.0) };
                    let release = Controls { formants: next_formants, glide_ms: 40.0, aspiration, voicing, ..base };
                    push(vot_ms, release, steps);
                }
            }
        }
        ranges.push((from, time));
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    const SR: f32 = 44_100.0;

    fn spoken_in(text: &str, notation: Notation) -> (Speaker, Vec<f32>) {
        let mut s = Speaker::new(SR);
        s.set(param::ROUGHNESS, 0.0);
        s.speak(text, notation).unwrap();
        let mut out = vec![0.0; s.length() as usize];
        s.render(&mut out);
        (s, out)
    }

    fn spoken(text: &str) -> (Speaker, Vec<f32>) {
        spoken_in(text, Notation::Arpabet)
    }

    fn spoken_ipa(text: &str) -> (Speaker, Vec<f32>) {
        spoken_in(text, Notation::Ipa)
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|x| x * x).sum::<f32>() / samples.len().max(1) as f32).sqrt()
    }

    fn power_at(samples: &[f32], freq: f32) -> f32 {
        let coef = 2.0 * (TAU * freq / SR).cos();
        let (mut s1, mut s2) = (0.0_f32, 0.0_f32);
        for &x in samples {
            let s0 = x + coef * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        (s1 * s1 + s2 * s2 - coef * s1 * s2) / samples.len() as f32
    }

    /// Total power in a band, from several frequencies across it.
    fn band(samples: &[f32], low: f32, high: f32) -> f32 {
        (0..16).map(|i| power_at(samples, low + (high - low) * i as f32 / 15.0)).sum()
    }

    /// The audio of item `n`, trimmed at each end so transitions don't count.
    fn part<'a>(s: &Speaker, out: &'a [f32], n: usize) -> &'a [f32] {
        let item = s.items()[n];
        let trim = ((item.end - item.start) / 5) as usize;
        &out[item.start as usize + trim..item.end as usize - trim]
    }

    #[test]
    fn every_phoneme_renders_in_range() {
        for p in ENGLISH.phonemes {
            let text = format!("ˈɑ {} ˈɑ .", p.ipa);
            let (_, out) = spoken_ipa(&text);
            assert!(out.iter().all(|x| x.is_finite() && x.abs() <= 1.0), "{}", p.ipa);
            assert!(rms(&out) > 0.01, "{} is silent", p.ipa);
        }
    }

    #[test]
    fn items_follow_the_text() {
        let (s, _) = spoken("HH AH0 L OW1 , W ER1 L D .");
        let items = s.items();
        assert_eq!(items.len(), 10);
        assert_eq!((items[1].offset, items[1].len), (3, 3));
        assert_eq!((items[4].offset, items[4].len), (13, 1));
        for pair in items.windows(2) {
            assert_eq!(pair[0].end, pair[1].start);
            assert!(pair[0].start < pair[0].end);
        }
        assert!(items[9].end < s.length());
    }

    #[test]
    fn unstressed_vowels_are_shorter() {
        let (s, _) = spoken("B AH1 B AH0 B AH1 .");
        let len = |n: usize| s.items()[n].end - s.items()[n].start;
        assert!(len(3) < len(1) * 7 / 10, "unstressed {}, stressed {}", len(3), len(1));
    }

    #[test]
    fn speaking_faster_takes_less_time() {
        let text = "S P IY1 CH , S IH1 N TH AH0 S AH0 S .";
        let mut slow = Speaker::new(SR);
        slow.speak(text, Notation::Arpabet).unwrap();
        let mut fast = Speaker::new(SR);
        fast.set(param::RATE, 2.0);
        fast.speak(text, Notation::Arpabet).unwrap();
        let ratio = fast.length() as f32 / slow.length() as f32;
        assert!((0.45..0.7).contains(&ratio), "fast is {ratio} of normal");
    }

    #[test]
    fn s_hisses_higher_than_sh() {
        let (s, out) = spoken("AA1 S AA1 SH AA1 .");
        let (hiss_s, hiss_sh) = (part(&s, &out, 1), part(&s, &out, 3));
        let tilt = |x: &[f32]| band(x, 4000.0, 8000.0) / band(x, 1500.0, 3500.0);
        assert!(tilt(hiss_s) > tilt(hiss_sh) * 3.0, "S {}, SH {}", tilt(hiss_s), tilt(hiss_sh));
    }

    #[test]
    fn fricatives_are_quieter_than_vowels_but_heard() {
        let (s, out) = spoken("AA1 S AA1 F AA1 .");
        let vowel = rms(part(&s, &out, 0));
        let hiss = rms(part(&s, &out, 1));
        let weak = rms(part(&s, &out, 3));
        assert!((0.1..0.7).contains(&(hiss / vowel)), "S is {} of the vowel", hiss / vowel);
        assert!(weak < hiss && weak / vowel > 0.02, "F is {} of the vowel", weak / vowel);
    }

    #[test]
    fn voiceless_closures_are_silent_and_bursts_are_not() {
        let mut s = Speaker::new(SR);
        s.speak("AA1 T AA1 .", Notation::Arpabet).unwrap();
        let mut out = vec![0.0; s.length() as usize];
        s.render(&mut out);
        let t = s.items()[1];
        let vowel = rms(part(&s, &out, 0));
        // The closure is the first part of T. Skip the vowel's ring-down at its start.
        let closure_end = t.start + (65.0 * SR / 1000.0) as u32;
        let closure = &out[(t.start + (25.0 * SR / 1000.0) as u32) as usize..closure_end as usize];
        let burst = &out[closure_end as usize..(closure_end + (10.0 * SR / 1000.0) as u32) as usize];
        assert!(rms(closure) < vowel * 0.05, "closure {} vs vowel {vowel}", rms(closure));
        assert!(rms(burst) > rms(closure) * 4.0, "burst {} vs closure {}", rms(burst), rms(closure));
    }

    #[test]
    fn nasals_hum_low() {
        let (s, out) = spoken("AA1 M AA1 .");
        let tilt = |x: &[f32]| band(x, 150.0, 400.0) / band(x, 1000.0, 3000.0);
        let (vowel, hum) = (part(&s, &out, 0), part(&s, &out, 1));
        assert!(tilt(hum) > tilt(vowel) * 3.0, "M {}, AA {}", tilt(hum), tilt(vowel));
        assert!(rms(hum) < rms(vowel));
    }

    #[test]
    fn diphthongs_move_between_targets() {
        let mut s = Speaker::new(SR);
        s.speak("AY1 .", Notation::Arpabet).unwrap();
        let ay = s.items()[0];
        let mut out = vec![0.0; (ay.start + (ay.end - ay.start) / 4) as usize];
        s.render(&mut out);
        let early = s.formant(1);
        let mut rest = vec![0.0; (ay.end - ay.start) as usize * 3 / 4];
        s.render(&mut rest);
        assert!(early < 1500.0 && s.formant(1) > 1850.0, "F2 from {early} to {}", s.formant(1));
    }

    #[test]
    fn velars_meet_the_vowel_that_follows() {
        let mut front = Speaker::new(SR);
        front.speak("K IY1", Notation::Arpabet).unwrap();
        let mut back = Speaker::new(SR);
        back.speak("K UW1", Notation::Arpabet).unwrap();
        let mut out = vec![0.0; (60.0 * SR / 1000.0) as usize];
        front.render(&mut out);
        back.render(&mut out);
        assert!(front.formant(1) > back.formant(1) + 500.0, "front {}, back {}", front.formant(1), back.formant(1));
    }

    #[test]
    fn questions_rise_and_statements_fall() {
        let end_pitch = |text: &str| {
            let mut s = Speaker::new(SR);
            s.speak(text, Notation::Arpabet).unwrap();
            let last = s.items()[1];
            let mut out = vec![0.0; last.end as usize];
            s.render(&mut out);
            s.pitch()
        };
        let question = end_pitch("Y UW1 ?");
        let statement = end_pitch("Y UW1 .");
        assert!(question > 125.0 && statement < 105.0, "question {question} Hz, statement {statement} Hz");
    }

    #[test]
    fn monotone_with_no_intonation() {
        let mut s = Speaker::new(SR);
        s.set(param::INTONATION, 0.0);
        s.speak("HH AW1 AA1 R Y UW1 ?", Notation::Arpabet).unwrap();
        let mut out = vec![0.0; s.length() as usize];
        let mut highest = 0.0_f32;
        let mut lowest = f32::MAX;
        for chunk in out.chunks_mut(256) {
            s.render(chunk);
            highest = highest.max(s.pitch());
            lowest = lowest.min(s.pitch());
        }
        assert!(highest - lowest < 1.0, "pitch moved from {lowest} to {highest}");
    }

    #[test]
    fn parse_errors_change_nothing() {
        let mut s = Speaker::new(SR);
        s.speak("B IY1", Notation::Arpabet).unwrap();
        let length = s.length();
        assert_eq!(s.speak("B QQ1", Notation::Arpabet), Err(ParseError { offset: 2, len: 3 }));
        assert_eq!(s.length(), length);
    }

    #[test]
    fn empty_text_is_a_short_silence() {
        let (s, out) = spoken("");
        assert!(s.items().is_empty());
        assert!(s.length() > 0 && rms(&out) < 1e-4);
    }

    #[test]
    fn ipa_and_arpabet_sound_the_same() {
        assert_eq!(spoken("HH AH0 L OW1 , W ER1 L D .").1, spoken_ipa("həˈloʊ, ˈwɝld.").1);
    }

    fn spoken_text(text: &str) -> (Speaker, Vec<f32>) {
        let mut s = Speaker::new(SR);
        s.set(param::ROUGHNESS, 0.0);
        s.speak_text(text, Some(crate::dictionary::english_for_tests())).unwrap();
        let mut out = vec![0.0; s.length() as usize];
        s.render(&mut out);
        (s, out)
    }

    #[test]
    fn reads_ordinary_text() {
        let (s, out) = spoken_text("Hello, world!");
        assert_eq!(s.transcript(), "həˈloʊ, ˈwɝld!");
        assert!(out.iter().all(|x| x.is_finite() && x.abs() <= 1.0) && rms(&out) > 0.01);
        // Every sound points at its word: "Hello" is bytes 0 to 5, "world" 7 to 12, and the marks follow them.
        let places: Vec<(u32, u32)> = s.items().iter().map(|i| (i.offset, i.len)).collect();
        assert_eq!(places, [(0, 5), (0, 5), (0, 5), (0, 5), (5, 1), (7, 5), (7, 5), (7, 5), (7, 5), (12, 1)]);
    }

    #[test]
    fn text_questions_rise_and_statements_fall() {
        let end_pitch = |text: &str| {
            let (mut s, _) = spoken_text("");
            s.speak_text(text, Some(crate::dictionary::english_for_tests())).unwrap();
            let last = s.items()[s.items().len() - 2];
            let mut out = vec![0.0; last.end as usize];
            s.render(&mut out);
            s.pitch()
        };
        let question = end_pitch("Is it raining?");
        let statement = end_pitch("It is raining.");
        assert!(question > 120.0 && statement < 100.0, "question {question} Hz, statement {statement} Hz");
    }

    #[test]
    fn text_without_a_dictionary_uses_spelling_rules() {
        let mut s = Speaker::new(SR);
        s.speak("Hello there.", Notation::Text).unwrap();
        assert!(s.length() > 0 && !s.transcript().is_empty());
        s.speak("HH AY1", Notation::Arpabet).unwrap();
        assert_eq!(s.transcript(), "");
    }

    /// F2 at the middle of item `n`.
    fn f2_at(text: &str, n: usize) -> f32 {
        let mut s = Speaker::new(SR);
        s.speak(text, Notation::Arpabet).unwrap();
        let item = s.items()[n];
        let mut out = vec![0.0; ((item.start + item.end) / 2) as usize];
        s.render(&mut out);
        s.formant(1)
    }

    fn item_len(text: &str, n: usize) -> u32 {
        let mut s = Speaker::new(SR);
        s.speak(text, Notation::Arpabet).unwrap();
        s.items()[n].end - s.items()[n].start
    }

    #[test]
    fn consonants_lean_toward_their_vowel() {
        // /b/ before "ee" points much higher in F2 than before "oo", as the tongue is already in place.
        let (front, back) = (f2_at("B IY1 .", 0), f2_at("B UW1 .", 0));
        assert!(front > back + 500.0, "B before IY {front}, before UW {back}");
        // /d/ holds its place more than /b/ does.
        let (d_front, d_back) = (f2_at("D IY1 .", 0), f2_at("D UW1 .", 0));
        assert!(d_front - d_back < front - back, "D moves {}, B {}", d_front - d_back, front - back);
    }

    #[test]
    fn weak_vowels_fall_short_of_their_targets() {
        // /ɪ/ between /b/s: F2 stays nearer its target stressed than unstressed, which leans toward the lips' low F2.
        let stressed = f2_at("B IH1 B .", 1);
        let unstressed = f2_at("B AH1 B IH0 B .", 3);
        assert!(stressed > unstressed + 100.0, "stressed {stressed}, unstressed {unstressed}");
    }

    #[test]
    fn vowels_are_shorter_before_voiceless_consonants() {
        assert!(item_len("B IH1 T .", 1) < item_len("B IH1 D .", 1) * 9 / 10);
    }

    #[test]
    fn consonants_are_shorter_in_clusters() {
        assert!(item_len("AA1 S T AA1 .", 1) < item_len("AA1 S AA1 .", 1));
    }

    #[test]
    fn the_tap_is_short_and_keeps_the_voice_on() {
        let mut s = Speaker::new(SR);
        s.set(param::ROUGHNESS, 0.0);
        s.speak("B AH1 T ER0 .", Notation::Arpabet).unwrap();
        let mut out = vec![0.0; s.length() as usize];
        s.render(&mut out);
        let tap = s.items()[2];
        let ms = (tap.end - tap.start) as f32 * 1000.0 / SR;
        assert!(ms < 35.0, "the tap lasts {ms} ms");
        let voiced = rms(&out[tap.start as usize..tap.end as usize]);
        assert!(voiced > rms(part(&s, &out, 1)) * 0.2, "the tap is {voiced}");
    }

    #[test]
    fn same_text_gives_the_same_sound() {
        assert_eq!(spoken("V OW1 K AH0 L , B IH1 N .").1, spoken("V OW1 K AH0 L , B IH1 N .").1);
    }
}
