//! Stage 2: speaks phonemes, written in IPA or (for English) ARPAbet. Each phoneme becomes one or more timed steps of
//! [`Controls`], and the voice glides between them, so transitions come from the same spring-like motion as the
//! vowel lab's glides. Nothing here depends on the language: the sounds come from the language's table, and the
//! steps from each sound's [`Kind`] and [`Place`].
//!
//! Intonation here is deliberately simple (stage 3 is prosody): pitch drifts down across a phrase, stressed vowels
//! rise, and the phrase ends falling, or rising for `?`.

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

    /// Prepares `text` to be rendered from its start. On an error, nothing changes.
    pub fn speak(&mut self, text: &str, notation: Notation) -> Result<(), ParseError> {
        let tokens = match notation {
            Notation::Ipa => self.language.parse_ipa(text)?,
            Notation::Arpabet => parse_arpabet(text)?,
        };
        self.speak_tokens(&tokens);
        Ok(())
    }

    /// Prepares phonemes that are already read, such as from a dictionary, to be rendered from their start.
    pub fn speak_tokens(&mut self, tokens: &[Token]) {
        let (steps, items) = plan(tokens, self.voice.sample_rate(), self.rate, self.intonation);
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

struct Phone {
    phoneme: &'static Phoneme,
    stress: u8,
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
                Token::Phone { phoneme, stress, offset, len } => Some(Phone { phoneme, stress, offset, len }),
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

    // Pitch: a gentle fall across the phrase, a lift on stressed vowels, and the phrase's ending.
    let mut pitch = vec![1.0_f32; phones.len()];
    let mut current = 1.0;
    for i in 0..phones.len() {
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
    }
    // Consonants before the first vowel take its pitch.
    if let Some(&first) = vowels.first() {
        for p in pitch.iter_mut().take(first) {
            *p = 1.0 + 0.10 * intonation;
        }
    }

    let next_vowel = |i: usize| phones[i + 1..].iter().find(|p| p.phoneme.is_vowel());
    let mut ranges = Vec::with_capacity(phones.len());
    let mut time = 0_u32;
    for (i, phone) in phones.iter().enumerate() {
        let p = phone.phoneme;
        let lengthen = if last_vowel.is_some_and(|v| i >= v) { FINAL_LENGTHENING } else { 1.0 };
        let sonorant_ms = |ms: f32| ms * lengthen / rate;
        let closure_ms = |ms: f32| ms / rate.powf(0.7);
        let next = phones.get(i + 1);
        // Where the formants head after a consonant: the next sound's start, or a neutral vowel.
        let next_formants = next.map_or(NEUTRAL, |n| n.phoneme.formants);
        let after_vowel = next_vowel(i).map_or(NEUTRAL, |n| n.phoneme.formants);
        let base = Controls { formants: p.formants, pitch_scale: pitch[i], ..Controls::SILENT };
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
                let ms = sonorant_ms(p.ms * stress);
                let glide_ms = match i.checked_sub(1).map(|j| phones[j].phoneme.kind) {
                    Some(Kind::Plosive | Kind::Affricate) => 45.0,
                    Some(Kind::Nasal) => 40.0,
                    Some(Kind::Fricative | Kind::Aspirate) => 50.0,
                    Some(Kind::Approximant) => 75.0,
                    Some(Kind::Vowel | Kind::Diphthong) => 90.0,
                    None => 40.0,
                };
                let vowel = Controls { formants: p.formants, glide_ms, voicing: 1.0, ..base };
                if p.kind == Kind::Diphthong {
                    push(ms * 0.4, vowel, steps);
                    push(ms * 0.6, Controls { formants: p.end, glide_ms: ms * 0.6, ..vowel }, steps);
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
            Kind::Plosive | Kind::Affricate => {
                let mut locus = p.formants;
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

    #[test]
    fn same_text_gives_the_same_sound() {
        assert_eq!(spoken("V OW1 K AH0 L , B IH1 N .").1, spoken("V OW1 K AH0 L , B IH1 N .").1);
    }
}
