//! American English: its sounds in IPA, and ARPAbet, the CMU Pronouncing Dictionary's way of writing them.
//!
//! Vowel formants are the stage 1 values (Hillenbrand et al. 1995, adult men). Consonant formants are loci: where
//! the formants point during the consonant, which shapes the transitions into and out of it. Lengths and noise
//! spectra are starting points in the spirit of Klatt's rules, to be tuned by ear.

use crate::language::Language;
use crate::phonemes::*;

pub const ENGLISH: Language = Language { code: "en-US", name: "English (US)", phonemes: &PHONEMES, aliases: &ALIASES };

/// The 39 sounds of the CMU dictionary, plus /ə/ and /ɚ/, the unstressed forms of /ʌ/ and /ɝ/.
pub const PHONEMES: [Phoneme; 41] = [
    vowel("ɑ", [768.0, 1333.0, 2522.0], 200.0),
    vowel("æ", [588.0, 1952.0, 2601.0], 190.0),
    vowel("ʌ", [623.0, 1200.0, 2550.0], 140.0),
    reduced("ə", NEUTRAL, 140.0),
    vowel("ɔ", [652.0, 997.0, 2538.0], 190.0),
    diphthong("aʊ", [768.0, 1333.0, 2522.0], [470.0, 1050.0, 2400.0], 240.0),
    diphthong("aɪ", [768.0, 1333.0, 2522.0], [430.0, 2000.0, 2650.0], 240.0),
    vowel("ɛ", [580.0, 1799.0, 2605.0], 140.0),
    vowel("ɝ", [474.0, 1379.0, 1710.0], 170.0),
    reduced("ɚ", [474.0, 1379.0, 1710.0], 170.0),
    diphthong("eɪ", [476.0, 2089.0, 2691.0], [400.0, 2250.0, 2850.0], 180.0),
    vowel("ɪ", [427.0, 2034.0, 2684.0], 120.0),
    vowel("i", [342.0, 2322.0, 3000.0], 150.0),
    diphthong("oʊ", [497.0, 910.0, 2459.0], [430.0, 1000.0, 2400.0], 180.0),
    diphthong("ɔɪ", [652.0, 997.0, 2538.0], [430.0, 2000.0, 2650.0], 250.0),
    vowel("ʊ", [469.0, 1122.0, 2434.0], 120.0),
    vowel("u", [378.0, 997.0, 2343.0], 160.0),
    plosive("b", Place::Labial, true, [250.0, 1000.0, 2150.0], 65.0, 12.0),
    affricate("tʃ", Place::Postalveolar, false, [300.0, 1840.0, 2750.0], SH_NOISE, 55.0, 85.0),
    plosive("d", Place::Alveolar, true, [250.0, 1700.0, 2600.0], 55.0, 15.0),
    fricative("ð", Place::Dental, true, [320.0, 1290.0, 2540.0], TH_NOISE, 50.0),
    fricative("f", Place::Labial, false, [340.0, 1100.0, 2080.0], F_NOISE, 100.0),
    plosive("ɡ", Place::Velar, true, [250.0, 2000.0, 2500.0], 55.0, 22.0),
    base("h", Kind::Aspirate, Place::Glottal, false, NEUTRAL, 65.0),
    affricate("dʒ", Place::Postalveolar, true, [300.0, 1840.0, 2750.0], SH_NOISE, 45.0, 60.0),
    plosive("k", Place::Velar, false, [250.0, 2000.0, 2500.0], 70.0, 65.0),
    approximant("l", Place::Alveolar, [310.0, 1050.0, 2880.0], 70.0),
    nasal("m", Place::Labial, [270.0, 1000.0, 2150.0], 900.0, 70.0),
    nasal("n", Place::Alveolar, [270.0, 1600.0, 2600.0], 1500.0, 70.0),
    nasal("ŋ", Place::Velar, [270.0, 2000.0, 2600.0], 3000.0, 75.0),
    plosive("p", Place::Labial, false, [250.0, 1000.0, 2150.0], 75.0, 50.0),
    approximant("ɹ", Place::Postalveolar, [310.0, 1060.0, 1380.0], 70.0),
    fricative("s", Place::Alveolar, false, [320.0, 1390.0, 2530.0], S_NOISE, 110.0),
    fricative("ʃ", Place::Postalveolar, false, [300.0, 1840.0, 2750.0], SH_NOISE, 120.0),
    plosive("t", Place::Alveolar, false, [250.0, 1700.0, 2600.0], 65.0, 60.0),
    fricative("θ", Place::Dental, false, [320.0, 1290.0, 2540.0], TH_NOISE, 100.0),
    fricative("v", Place::Labial, true, [340.0, 1100.0, 2080.0], F_NOISE, 60.0),
    approximant("w", Place::Labial, [290.0, 610.0, 2150.0], 65.0),
    approximant("j", Place::Palatal, [260.0, 2070.0, 3020.0], 60.0),
    fricative("z", Place::Alveolar, true, [320.0, 1390.0, 2530.0], S_NOISE, 75.0),
    fricative("ʒ", Place::Postalveolar, true, [300.0, 1840.0, 2750.0], SH_NOISE, 70.0),
];

/// Other common ways to write these sounds: plain letters, tie bars and ligatures, British symbols, the flap and
/// dark L, and syllabic consonants.
const ALIASES: [(&str, &[&str]); 22] = [
    ("r", &["ɹ"]),
    ("g", &["ɡ"]),
    ("e", &["eɪ"]),
    ("o", &["oʊ"]),
    ("ʧ", &["tʃ"]),
    ("ʤ", &["dʒ"]),
    ("t͡ʃ", &["tʃ"]),
    ("d͡ʒ", &["dʒ"]),
    ("ɜ", &["ɝ"]),
    ("ɒ", &["ɑ"]),
    ("ɐ", &["ʌ"]),
    ("əʊ", &["oʊ"]),
    ("ɪə", &["ɪ", "ə"]),
    ("eə", &["ɛ", "ə"]),
    ("ɛə", &["ɛ", "ə"]),
    ("ʊə", &["ʊ", "ə"]),
    ("ɫ", &["l"]),
    ("ɾ", &["d"]),
    ("n̩", &["ə", "n"]),
    ("l̩", &["ə", "l"]),
    ("m̩", &["ə", "m"]),
    ("ʍ", &["w"]),
];

/// One ARPAbet symbol and the English sound it writes.
pub struct Arpabet {
    pub symbol: &'static str,
    /// The sound when stressed, or for consonants, always.
    pub ipa: &'static str,
    /// The sound with stress 0. Only `AH` and `ER` differ: they become /ə/ and /ɚ/.
    pub unstressed: &'static str,
    /// The CMU dictionary's example word.
    pub example: &'static str,
}

const fn arpa(symbol: &'static str, ipa: &'static str, example: &'static str) -> Arpabet {
    Arpabet { symbol, ipa, unstressed: ipa, example }
}

pub const ARPABET: [Arpabet; 39] = [
    arpa("AA", "ɑ", "odd"),
    arpa("AE", "æ", "at"),
    Arpabet { symbol: "AH", ipa: "ʌ", unstressed: "ə", example: "hut" },
    arpa("AO", "ɔ", "ought"),
    arpa("AW", "aʊ", "cow"),
    arpa("AY", "aɪ", "hide"),
    arpa("EH", "ɛ", "Ed"),
    Arpabet { symbol: "ER", ipa: "ɝ", unstressed: "ɚ", example: "hurt" },
    arpa("EY", "eɪ", "ate"),
    arpa("IH", "ɪ", "it"),
    arpa("IY", "i", "eat"),
    arpa("OW", "oʊ", "oat"),
    arpa("OY", "ɔɪ", "toy"),
    arpa("UH", "ʊ", "hood"),
    arpa("UW", "u", "two"),
    arpa("B", "b", "be"),
    arpa("CH", "tʃ", "cheese"),
    arpa("D", "d", "dee"),
    arpa("DH", "ð", "thee"),
    arpa("F", "f", "fee"),
    arpa("G", "ɡ", "green"),
    arpa("HH", "h", "he"),
    arpa("JH", "dʒ", "gee"),
    arpa("K", "k", "key"),
    arpa("L", "l", "lee"),
    arpa("M", "m", "me"),
    arpa("N", "n", "knee"),
    arpa("NG", "ŋ", "ping"),
    arpa("P", "p", "pee"),
    arpa("R", "ɹ", "read"),
    arpa("S", "s", "sea"),
    arpa("SH", "ʃ", "she"),
    arpa("T", "t", "tea"),
    arpa("TH", "θ", "theta"),
    arpa("V", "v", "vee"),
    arpa("W", "w", "we"),
    arpa("Y", "j", "yield"),
    arpa("Z", "z", "zee"),
    arpa("ZH", "ʒ", "seizure"),
];

/// Reads ARPAbet separated by spaces, such as `HH AH0 L OW1 , W ER1 L D .`, as English sounds. Case doesn't matter.
/// A vowel without a stress number counts as stressed. Consonants take no stress number. `,` is a short pause, and
/// `.`, `!`, and `?` end a sentence.
pub fn parse_arpabet(text: &str) -> Result<Vec<Token>, ParseError> {
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
            let arpabet = ARPABET.iter().find(|a| a.symbol.eq_ignore_ascii_case(symbol)).ok_or(error)?;
            let sound = ENGLISH.find(arpabet.ipa).expect("every ARPAbet symbol is an English sound");
            if stress.is_some() && !sound.is_vowel() {
                return Err(error);
            }
            let stress = if sound.is_vowel() { stress.unwrap_or(1) } else { 0 };
            let ipa = if stress == 0 { arpabet.unstressed } else { arpabet.ipa };
            let phoneme = ENGLISH.find(ipa).expect("every ARPAbet symbol is an English sound");
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

    /// Each token as IPA with its stress for vowels, or the pause mark.
    fn read(tokens: Vec<Token>) -> Vec<String> {
        tokens
            .iter()
            .map(|t| match t {
                Token::Phone { phoneme, stress, .. } if phoneme.is_vowel() => format!("{}{}", phoneme.ipa, stress),
                Token::Phone { phoneme, .. } => phoneme.ipa.to_string(),
                Token::Pause { mark, .. } => mark.to_string(),
            })
            .collect()
    }

    fn arpabet(text: &str) -> Vec<String> {
        read(parse_arpabet(text).unwrap())
    }

    fn ipa(text: &str) -> Vec<String> {
        read(ENGLISH.parse_ipa(text).unwrap())
    }

    #[test]
    fn has_each_sound_once() {
        for p in &PHONEMES {
            assert_eq!(PHONEMES.iter().filter(|q| q.ipa == p.ipa).count(), 1, "{}", p.ipa);
            assert!(p.ms > 0.0 && p.formants.iter().all(|&f| f > 0.0), "{}", p.ipa);
            assert_eq!(p.place == Place::None, p.is_vowel(), "{}", p.ipa);
        }
        assert_eq!(PHONEMES.iter().filter(|p| p.is_vowel()).count(), 17);
    }

    #[test]
    fn every_arpabet_symbol_and_alias_is_a_sound() {
        assert_eq!(ARPABET.len(), 39);
        for a in &ARPABET {
            assert!(ENGLISH.find(a.ipa).is_some() && ENGLISH.find(a.unstressed).is_some(), "{}", a.symbol);
        }
        for (alias, sounds) in &ALIASES {
            assert!(sounds.iter().all(|s| ENGLISH.find(s).is_some()), "{alias}");
            assert!(ENGLISH.find(alias).is_none(), "{alias} is already a sound");
        }
    }

    #[test]
    fn reads_arpabet_stress_and_pauses() {
        assert_eq!(arpabet("HH AH0 L OW1, W ER1 L D."), ["h", "ə0", "l", "oʊ1", ",", "w", "ɝ1", "l", "d", "."]);
    }

    #[test]
    fn unstressed_ah_and_er_are_reduced() {
        assert_eq!(arpabet("AH0 AH1 AH2 AH ER0 ER1"), ["ə0", "ʌ1", "ʌ2", "ʌ1", "ɚ0", "ɝ1"]);
    }

    #[test]
    fn arpabet_ignores_case_and_extra_space() {
        assert_eq!(arpabet("  hh  ah0\nl\tow1  "), ["h", "ə0", "l", "oʊ1"]);
    }

    #[test]
    fn arpabet_vowels_without_stress_are_stressed() {
        assert_eq!(arpabet("B IY"), ["b", "i1"]);
    }

    #[test]
    fn arpabet_keeps_where_each_phoneme_is() {
        let tokens = parse_arpabet("HH AY1 ?").unwrap();
        assert_eq!(tokens[1], Token::Phone { phoneme: ENGLISH.find("aɪ").unwrap(), stress: 1, offset: 3, len: 3 });
        assert_eq!(tokens[2], Token::Pause { mark: '?', offset: 7 });
    }

    #[test]
    fn arpabet_points_at_what_it_cannot_read() {
        assert_eq!(parse_arpabet("HH XX1 L"), Err(ParseError { offset: 3, len: 3 }));
        assert_eq!(parse_arpabet("HH AH3"), Err(ParseError { offset: 3, len: 3 }));
        assert_eq!(parse_arpabet("S1 IY"), Err(ParseError { offset: 0, len: 2 }));
        assert_eq!(parse_arpabet("HH - L"), Err(ParseError { offset: 3, len: 1 }));
        assert_eq!(parse_arpabet("HH é"), Err(ParseError { offset: 3, len: 2 }));
    }

    #[test]
    fn empty_text_reads_as_nothing() {
        assert_eq!(parse_arpabet("  "), Ok(vec![]));
        assert_eq!(ENGLISH.parse_ipa("  "), Ok(vec![]));
    }

    #[test]
    fn stops_at_the_length_limit() {
        let long = "AH ".repeat(MAX_TOKENS + 1);
        assert_eq!(parse_arpabet(&long).unwrap_err().offset, MAX_TOKENS * 3);
        let long = "ə ".repeat(MAX_TOKENS + 1);
        assert_eq!(ENGLISH.parse_ipa(&long).unwrap_err().offset, MAX_TOKENS * 3);
    }

    #[test]
    fn ipa_reads_words_stress_and_pauses() {
        assert_eq!(ipa("həˈloʊ, ˈwɝld."), ["h", "ə0", "l", "oʊ1", ",", "w", "ɝ1", "l", "d", "."]);
        assert_eq!(ipa("ˌɛkspləˈneɪʃən"), ["ɛ2", "k", "s", "p", "l", "ə0", "n", "eɪ1", "ʃ", "ə0", "n"]);
    }

    #[test]
    fn ipa_matches_the_longest_sound() {
        assert_eq!(ipa("ˈtʃaɪ"), ["tʃ", "aɪ1"]);
        assert_eq!(ipa("ˈhaʊs"), ["h", "aʊ1", "s"]);
    }

    #[test]
    fn ipa_stresses_unmarked_words_on_their_first_full_vowel() {
        assert_eq!(ipa("bi"), ["b", "i1"]);
        assert_eq!(ipa("əbaʊt"), ["ə0", "b", "aʊ1", "t"]);
        assert_eq!(ipa("ðə kæt"), ["ð", "ə0", "k", "æ1", "t"]);
    }

    #[test]
    fn ipa_reads_common_alternatives() {
        assert_eq!(ipa("/ˈrɛd/"), ["ɹ", "ɛ1", "d"]);
        assert_eq!(ipa("[ˈgoʊ]"), ["ɡ", "oʊ1"]);
        assert_eq!(ipa("ˈt͡ʃiːz"), ["tʃ", "i1", "z"]);
        assert_eq!(ipa("ˈbɜːd"), ["b", "ɝ1", "d"]);
        assert_eq!(ipa("ˈbʌtn̩"), ["b", "ʌ1", "t", "ə0", "n"]);
        assert_eq!(ipa("ˈbʌɾɚ"), ["b", "ʌ1", "d", "ɚ0"]);
    }

    #[test]
    fn ipa_dots_between_sounds_are_syllable_breaks() {
        assert_eq!(ipa("ˈwɔ.tɚ."), ["w", "ɔ1", "t", "ɚ0", "."]);
        assert_eq!(ipa("ˈhaɪ | ˈjæ ‖"), ["h", "aɪ1", ",", "j", "æ1", "."]);
    }

    #[test]
    fn ipa_keeps_where_each_sound_is() {
        let tokens = ENGLISH.parse_ipa("ˈtʃaɪ ?").unwrap();
        // ˈ is two bytes, tʃ three, aɪ three.
        assert_eq!(tokens[0], Token::Phone { phoneme: ENGLISH.find("tʃ").unwrap(), stress: 0, offset: 2, len: 3 });
        assert_eq!(tokens[1], Token::Phone { phoneme: ENGLISH.find("aɪ").unwrap(), stress: 1, offset: 5, len: 3 });
        assert_eq!(tokens[2], Token::Pause { mark: '?', offset: 9 });
    }

    #[test]
    fn ipa_points_at_what_it_cannot_read() {
        assert_eq!(ENGLISH.parse_ipa("ˈbʌʔn"), Err(ParseError { offset: 5, len: 2 }));
        assert_eq!(ENGLISH.parse_ipa("hi x"), Err(ParseError { offset: 3, len: 1 }));
    }
}
