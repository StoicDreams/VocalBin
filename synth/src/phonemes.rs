//! The 39 ARPAbet phonemes of the CMU Pronouncing Dictionary, and the parser for text written with them.
//!
//! Vowel formants are the stage 1 values (Hillenbrand et al. 1995, adult men). Consonant formants are loci: where
//! the formants point during the consonant, which shapes the transitions into and out of it. Lengths and noise
//! spectra are starting points in the spirit of Klatt's rules, to be tuned by ear.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Vowel,
    /// A vowel that moves from one target to another, as in `AY` (hide).
    Diphthong,
    /// `L`, `R`, `W`, `Y`: voiced, vowel-like, with slower transitions.
    Approximant,
    /// `M`, `N`, `NG`: voiced through the nose, with an extra pole and zero.
    Nasal,
    /// Hiss from a narrow gap: `S`, `SH`, `F`, `TH`, and their voiced pairs.
    Fricative,
    /// A closure, a burst, then aspiration or voicing: `P`, `T`, `K`, `B`, `D`, `G`.
    Plosive,
    /// A plosive released into a fricative: `CH`, `JH`.
    Affricate,
    /// `HH`: breath through the shape of the vowel that follows.
    Aspirate,
}

#[derive(Debug, PartialEq)]
pub struct Phoneme {
    /// The ARPAbet symbol, without a stress number.
    pub symbol: &'static str,
    pub ipa: &'static str,
    /// The CMU dictionary's example word.
    pub example: &'static str,
    pub kind: Kind,
    pub voiced: bool,
    /// F1 to F3 in Hz: the vowel target, or the consonant's locus.
    pub formants: [f32; 3],
    /// Where a diphthong ends. The same as `formants` for every other phoneme.
    pub end: [f32; 3],
    /// Length in ms at normal speed. For plosives and affricates, the closure.
    pub ms: f32,
    /// Plosives: the voice onset time after the burst. Affricates: the length of the fricative part.
    pub release_ms: f32,
    /// Fricative hiss, or a plosive's burst, as levels in the parallel bands F2, F3, F4, F5, F6, and bypass.
    pub noise: [f32; 6],
    /// Nasals: the nasal zero in Hz.
    pub nasal_zero: f32,
}

const QUIET: [f32; 6] = [0.0; 6];
/// The hiss of `S` and `Z`: high and strong, from F5 and F6.
const S_NOISE: [f32; 6] = [0.0, 0.0, 0.1, 0.4, 1.0, 0.1];
/// The hiss of `SH`, `ZH`, `CH`, and `JH`: lower, peaking at F3, and the loudest fricative.
const SH_NOISE: [f32; 6] = [0.6, 2.0, 1.2, 0.6, 0.2, 0.0];
/// The weak, flat hiss of `F` and `V`.
const F_NOISE: [f32; 6] = [0.0, 0.0, 0.0, 0.025, 0.04, 0.09];
/// The weak hiss of `TH` and `DH`.
const TH_NOISE: [f32; 6] = [0.0, 0.0, 0.0, 0.04, 0.05, 0.075];
/// Bursts: lips are flat and weak, the tongue tip is high, and the tongue back is compact around F2 and F3.
const LABIAL_BURST: [f32; 6] = [0.0, 0.0, 0.0, 0.0, 0.2, 0.6];
const ALVEOLAR_BURST: [f32; 6] = [0.0, 0.0, 0.3, 0.6, 1.0, 0.3];
const VELAR_BURST: [f32; 6] = [1.0, 0.7, 0.3, 0.0, 0.0, 0.0];

const fn base(symbol: &'static str, ipa: &'static str, example: &'static str, kind: Kind, voiced: bool, formants: [f32; 3], ms: f32) -> Phoneme {
    Phoneme { symbol, ipa, example, kind, voiced, formants, end: formants, ms, release_ms: 0.0, noise: QUIET, nasal_zero: 0.0 }
}

const fn vowel(symbol: &'static str, ipa: &'static str, example: &'static str, formants: [f32; 3], ms: f32) -> Phoneme {
    base(symbol, ipa, example, Kind::Vowel, true, formants, ms)
}

const fn diphthong(symbol: &'static str, ipa: &'static str, example: &'static str, formants: [f32; 3], end: [f32; 3], ms: f32) -> Phoneme {
    Phoneme { end, ..base(symbol, ipa, example, Kind::Diphthong, true, formants, ms) }
}

const fn fricative(symbol: &'static str, ipa: &'static str, example: &'static str, voiced: bool, formants: [f32; 3], noise: [f32; 6], ms: f32) -> Phoneme {
    Phoneme { noise, ..base(symbol, ipa, example, Kind::Fricative, voiced, formants, ms) }
}

const fn plosive(symbol: &'static str, ipa: &'static str, example: &'static str, voiced: bool, formants: [f32; 3], burst: [f32; 6], closure_ms: f32, vot_ms: f32) -> Phoneme {
    Phoneme { noise: burst, release_ms: vot_ms, ..base(symbol, ipa, example, Kind::Plosive, voiced, formants, closure_ms) }
}

const fn nasal(symbol: &'static str, ipa: &'static str, example: &'static str, formants: [f32; 3], zero: f32, ms: f32) -> Phoneme {
    Phoneme { nasal_zero: zero, ..base(symbol, ipa, example, Kind::Nasal, true, formants, ms) }
}

pub const SCHWA: [f32; 3] = [500.0, 1500.0, 2500.0];

pub const PHONEMES: [Phoneme; 39] = [
    vowel("AA", "ɑ", "odd", [768.0, 1333.0, 2522.0], 200.0),
    vowel("AE", "æ", "at", [588.0, 1952.0, 2601.0], 190.0),
    vowel("AH", "ʌ", "hut", [623.0, 1200.0, 2550.0], 140.0),
    vowel("AO", "ɔ", "ought", [652.0, 997.0, 2538.0], 190.0),
    diphthong("AW", "aʊ", "cow", [768.0, 1333.0, 2522.0], [470.0, 1050.0, 2400.0], 240.0),
    diphthong("AY", "aɪ", "hide", [768.0, 1333.0, 2522.0], [430.0, 2000.0, 2650.0], 240.0),
    vowel("EH", "ɛ", "Ed", [580.0, 1799.0, 2605.0], 140.0),
    vowel("ER", "ɝ", "hurt", [474.0, 1379.0, 1710.0], 170.0),
    diphthong("EY", "eɪ", "ate", [476.0, 2089.0, 2691.0], [400.0, 2250.0, 2850.0], 180.0),
    vowel("IH", "ɪ", "it", [427.0, 2034.0, 2684.0], 120.0),
    vowel("IY", "i", "eat", [342.0, 2322.0, 3000.0], 150.0),
    diphthong("OW", "oʊ", "oat", [497.0, 910.0, 2459.0], [430.0, 1000.0, 2400.0], 180.0),
    diphthong("OY", "ɔɪ", "toy", [652.0, 997.0, 2538.0], [430.0, 2000.0, 2650.0], 250.0),
    vowel("UH", "ʊ", "hood", [469.0, 1122.0, 2434.0], 120.0),
    vowel("UW", "u", "two", [378.0, 997.0, 2343.0], 160.0),
    plosive("B", "b", "be", true, [250.0, 1000.0, 2150.0], LABIAL_BURST, 65.0, 12.0),
    Phoneme { release_ms: 85.0, ..base("CH", "tʃ", "cheese", Kind::Affricate, false, [300.0, 1840.0, 2750.0], 55.0) },
    plosive("D", "d", "dee", true, [250.0, 1700.0, 2600.0], ALVEOLAR_BURST, 55.0, 15.0),
    fricative("DH", "ð", "thee", true, [320.0, 1290.0, 2540.0], TH_NOISE, 50.0),
    fricative("F", "f", "fee", false, [340.0, 1100.0, 2080.0], F_NOISE, 100.0),
    plosive("G", "ɡ", "green", true, [250.0, 2000.0, 2500.0], VELAR_BURST, 55.0, 22.0),
    base("HH", "h", "he", Kind::Aspirate, false, SCHWA, 65.0),
    Phoneme { release_ms: 60.0, ..base("JH", "dʒ", "gee", Kind::Affricate, true, [300.0, 1840.0, 2750.0], 45.0) },
    plosive("K", "k", "key", false, [250.0, 2000.0, 2500.0], VELAR_BURST, 70.0, 65.0),
    base("L", "l", "lee", Kind::Approximant, true, [310.0, 1050.0, 2880.0], 70.0),
    nasal("M", "m", "me", [270.0, 1000.0, 2150.0], 900.0, 70.0),
    nasal("N", "n", "knee", [270.0, 1600.0, 2600.0], 1500.0, 70.0),
    nasal("NG", "ŋ", "ping", [270.0, 2000.0, 2600.0], 3000.0, 75.0),
    plosive("P", "p", "pee", false, [250.0, 1000.0, 2150.0], LABIAL_BURST, 75.0, 50.0),
    base("R", "ɹ", "read", Kind::Approximant, true, [310.0, 1060.0, 1380.0], 70.0),
    fricative("S", "s", "sea", false, [320.0, 1390.0, 2530.0], S_NOISE, 110.0),
    fricative("SH", "ʃ", "she", false, [300.0, 1840.0, 2750.0], SH_NOISE, 120.0),
    plosive("T", "t", "tea", false, [250.0, 1700.0, 2600.0], ALVEOLAR_BURST, 65.0, 60.0),
    fricative("TH", "θ", "theta", false, [320.0, 1290.0, 2540.0], TH_NOISE, 100.0),
    fricative("V", "v", "vee", true, [340.0, 1100.0, 2080.0], F_NOISE, 60.0),
    base("W", "w", "we", Kind::Approximant, true, [290.0, 610.0, 2150.0], 65.0),
    base("Y", "j", "yield", Kind::Approximant, true, [260.0, 2070.0, 3020.0], 60.0),
    fricative("Z", "z", "zee", true, [320.0, 1390.0, 2530.0], S_NOISE, 75.0),
    fricative("ZH", "ʒ", "seizure", true, [300.0, 1840.0, 2750.0], SH_NOISE, 70.0),
];

/// The hiss of an affricate's fricative part.
pub const AFFRICATE_NOISE: [f32; 6] = SH_NOISE;
/// The burst that releases an affricate's closure.
pub const AFFRICATE_BURST: [f32; 6] = ALVEOLAR_BURST;

pub fn find(symbol: &str) -> Option<&'static Phoneme> {
    PHONEMES.iter().find(|p| p.symbol.eq_ignore_ascii_case(symbol))
}

impl Phoneme {
    pub fn is_vowel(&self) -> bool {
        matches!(self.kind, Kind::Vowel | Kind::Diphthong)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Token {
    /// A phoneme, with its stress for vowels (0 none, 1 primary, 2 secondary), and where it is in the text.
    Phone { phoneme: &'static Phoneme, stress: u8, offset: usize, len: usize },
    /// `,` is a short pause. `.`, `!`, and `?` end a sentence, and `?` ends it with a rise.
    Pause { mark: char, offset: usize },
}

/// Where in the text, in bytes, the first thing that isn't ARPAbet is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub offset: usize,
    pub len: usize,
}

/// The most phonemes and pauses one text may have.
pub const MAX_TOKENS: usize = 2000;

/// Reads ARPAbet separated by spaces, such as `HH AH0 L OW1 , W ER1 L D .`. Case doesn't matter. A vowel without a
/// stress number counts as stressed. Consonants take no stress number.
pub fn parse(text: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some(&(offset, c)) = chars.peek() {
        if tokens.len() >= MAX_TOKENS && !c.is_whitespace() {
            return Err(ParseError { offset, len: text.len() - offset });
        }
        if c.is_whitespace() {
            chars.next();
        } else if matches!(c, ',' | '.' | '!' | '?') {
            chars.next();
            tokens.push(Token::Pause { mark: c, offset });
        } else if c.is_ascii_alphanumeric() {
            let mut end = offset;
            while let Some(&(i, c)) = chars.peek() {
                if !c.is_ascii_alphanumeric() {
                    break;
                }
                end = i + c.len_utf8();
                chars.next();
            }
            let word = &text[offset..end];
            let error = ParseError { offset, len: end - offset };
            let (symbol, stress) = match word.as_bytes()[word.len() - 1] {
                digit @ b'0'..=b'2' => (&word[..word.len() - 1], Some(digit - b'0')),
                _ => (word, None),
            };
            let phoneme = find(symbol).ok_or(error)?;
            if stress.is_some() && !phoneme.is_vowel() {
                return Err(error);
            }
            let stress = if phoneme.is_vowel() { stress.unwrap_or(1) } else { 0 };
            tokens.push(Token::Phone { phoneme, stress, offset, len: end - offset });
        } else {
            return Err(ParseError { offset, len: c.len_utf8() });
        }
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn symbols(text: &str) -> Vec<String> {
        parse(text)
            .unwrap()
            .iter()
            .map(|t| match t {
                Token::Phone { phoneme, stress, .. } if phoneme.is_vowel() => format!("{}{}", phoneme.symbol, stress),
                Token::Phone { phoneme, .. } => phoneme.symbol.to_string(),
                Token::Pause { mark, .. } => mark.to_string(),
            })
            .collect()
    }

    #[test]
    fn has_every_cmu_phoneme_once() {
        assert_eq!(PHONEMES.len(), 39);
        for p in &PHONEMES {
            assert_eq!(PHONEMES.iter().filter(|q| q.symbol == p.symbol).count(), 1, "{}", p.symbol);
            assert!(p.ms > 0.0 && p.formants.iter().all(|&f| f > 0.0), "{}", p.symbol);
        }
        assert_eq!(PHONEMES.iter().filter(|p| p.is_vowel()).count(), 15);
    }

    #[test]
    fn reads_phonemes_stress_and_pauses() {
        assert_eq!(symbols("HH AH0 L OW1, W ER1 L D."), ["HH", "AH0", "L", "OW1", ",", "W", "ER1", "L", "D", "."]);
    }

    #[test]
    fn ignores_case_and_extra_space() {
        assert_eq!(symbols("  hh  ah0\nl\tow1  "), ["HH", "AH0", "L", "OW1"]);
    }

    #[test]
    fn treats_a_vowel_without_stress_as_stressed() {
        assert_eq!(symbols("B IY"), ["B", "IY1"]);
    }

    #[test]
    fn keeps_where_each_phoneme_is() {
        let tokens = parse("HH AY1 ?").unwrap();
        assert_eq!(tokens[1], Token::Phone { phoneme: find("AY").unwrap(), stress: 1, offset: 3, len: 3 });
        assert_eq!(tokens[2], Token::Pause { mark: '?', offset: 7 });
    }

    #[test]
    fn points_at_what_it_cannot_read() {
        assert_eq!(parse("HH XX1 L"), Err(ParseError { offset: 3, len: 3 }));
        assert_eq!(parse("HH AH3"), Err(ParseError { offset: 3, len: 3 }));
        assert_eq!(parse("S1 IY"), Err(ParseError { offset: 0, len: 2 }));
        assert_eq!(parse("HH - L"), Err(ParseError { offset: 3, len: 1 }));
        assert_eq!(parse("HH é"), Err(ParseError { offset: 3, len: 2 }));
    }

    #[test]
    fn reads_nothing_from_empty_text() {
        assert_eq!(parse("  "), Ok(vec![]));
    }

    #[test]
    fn stops_at_the_length_limit() {
        let long = "AH ".repeat(MAX_TOKENS + 1);
        assert_eq!(parse(&long).unwrap_err().offset, MAX_TOKENS * 3);
    }
}
