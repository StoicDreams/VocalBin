//! Letter-to-sound rules, for words the dictionary doesn't have. Written in the style of the Naval Research
//! Laboratory's rules (Elovitz, Johnson, McHugh, and Shore, 1976): each rule rewrites a run of letters as sounds when
//! the letters around it match, and the first rule that matches wins. Then a stress rule picks the stressed vowel.
//!
//! In a rule's context:
//!
//! - ` ` is the edge of the word.
//! - `#` is one or more vowels (A, E, I, O, U), `:` zero or more consonants, and `^` one consonant.
//! - `.` is a voiced consonant (B D G J L M N R V W Z), `+` a front vowel (E I Y), and `@` a consonant that makes a
//!   following long U sound "oo" (T S R D L Z N J TH CH SH).
//! - `&` is a sibilant (S C G Z X J CH SH), and `%` (after the letters only) a suffix: ER E ES ED ING ELY.
//!
//! Sounds are written in ARPAbet without stress numbers. The stress rule adds them.

use super::{ARPABET, ENGLISH};
use crate::dictionary::Sound;

type Rule = (&'static str, &'static str, &'static str, &'static str);

#[rustfmt::skip]
const RULES: &[Rule] = &[
    // A
    (" ", "A", " ", "AH"), ("", "A", " ", "AH"), (" ", "ARE", " ", "AA R"), (" ", "AR", "O", "AH R"), ("", "AR", "#", "EH R"),
    ("^", "AS", "#", "EY S"), ("", "A", "WA", "AH"), ("", "AW", "", "AO"), (" :", "ANY", "", "EH N IY"),
    ("", "A", "^+#", "EY"), ("#:", "ALLY", "", "AH L IY"), (" ", "AL", "#", "AH L"), ("", "AGAIN", "", "AH G EH N"),
    ("#:", "AG", "E", "IH JH"), ("", "A", "^+:#", "AE"), (" :", "A", "^+ ", "EY"), ("", "A", "^%", "EY"),
    (" ", "ARR", "", "AH R"), ("", "ARR", "", "AE R"), (" :", "AR", " ", "AA R"), ("", "AR", " ", "ER"), ("", "AR", "", "AA R"),
    ("", "AIR", "", "EH R"), ("", "AI", "", "EY"), ("", "AY", "", "EY"), ("", "AU", "", "AO"), ("#:", "AL", " ", "AH L"),
    ("#:", "ALS", " ", "AH L Z"), ("", "ALK", "", "AO K"), ("", "AL", "^", "AO L"), (" :", "ABLE", "", "EY B AH L"),
    ("", "ABLE", "", "AH B AH L"), ("", "ANG", "+", "EY N JH"), ("", "ATION", "", "EY SH AH N"), ("", "A", "", "AE"),
    // B
    (" ", "BE", "^#", "B IH"), ("", "BEING", "", "B IY IH NG"), (" ", "BOTH", " ", "B OW TH"), (" ", "BUS", "#", "B IH Z"),
    ("", "BUIL", "", "B IH L"), ("M", "B", " ", ""), ("", "BB", "", "B"), ("", "B", "", "B"),
    // C
    (" ", "CH", "^", "K"), ("^E", "CH", "", "K"), ("", "CH", "", "CH"), (" S", "CI", "#", "S AY"), ("", "CI", "A", "SH"),
    ("", "CI", "O", "SH"), ("", "CI", "EN", "SH"), ("", "CC", "+", "K S"), ("", "C", "+", "S"), ("", "CK", "", "K"),
    ("", "COM", "%", "K AH M"), ("", "CC", "", "K"), ("", "C", "", "K"),
    // D
    ("#:", "DED", " ", "D IH D"), (".E", "D", " ", "D"), ("#^:E", "D", " ", "T"), (" ", "DE", "^#", "D IH"),
    (" ", "DO", " ", "D UW"), (" ", "DOES", "", "D AH Z"), (" ", "DOING", "", "D UW IH NG"), (" ", "DOW", "", "D AW"),
    ("", "DU", "A", "JH UW"), ("", "DGE", "", "JH"), ("", "DD", "", "D"), ("", "D", "", "D"),
    // E
    ("#:", "E", " ", ""), ("'^:", "E", " ", ""), (" :", "E", " ", "IY"), ("#", "ED", " ", "D"), ("#:", "E", "D ", ""),
    ("", "EV", "ER", "EH V"), ("", "E", "^%", "IY"), ("", "ERI", "#", "IY R IY"), ("", "ERI", "", "EH R IH"),
    ("#:", "ER", "#", "ER"), ("", "ER", "#", "EH R"), ("", "ER", "", "ER"), (" ", "EVEN", "", "IY V EH N"),
    ("#:", "E", "W", ""), ("@", "EW", "", "UW"), ("", "EW", "", "Y UW"), ("", "E", "O", "IY"), ("#:&", "ES", " ", "IH Z"),
    ("#:", "E", "S ", ""), ("#:", "ELY", " ", "L IY"), ("#:", "EMENT", "", "M EH N T"), ("", "EFUL", "", "F UH L"),
    ("", "EE", "", "IY"), ("", "EARN", "", "ER N"), (" ", "EAR", "^", "ER"), ("", "EAD", "", "EH D"), ("#:", "EA", " ", "IY AH"),
    ("", "EA", "SU", "EH"), ("", "EA", "", "IY"), ("", "EIGH", "", "EY"), ("", "EI", "", "IY"), (" ", "EYE", "", "AY"),
    ("", "EY", "", "IY"), ("", "EU", "", "Y UW"), ("", "E", "", "EH"),
    // F
    ("", "FUL", "", "F UH L"), ("", "FF", "", "F"), ("", "F", "", "F"),
    // G
    ("", "GIV", "", "G IH V"), (" ", "G", "I^", "G"), ("", "GE", "T", "G EH"), ("SU", "GGES", "", "G JH EH S"),
    ("", "GG", "", "G"), (" B#", "G", "", "G"), ("", "G", "+", "JH"), ("", "GREAT", "", "G R EY T"), ("#", "GH", "", ""),
    (" ", "GN", "", "N"), ("", "GN", " ", "N"), ("", "G", "", "G"),
    // H
    (" ", "HAV", "", "HH AE V"), (" ", "HERE", "", "HH IY R"), (" ", "HOUR", "", "AW ER"), ("", "HOW", "", "HH AW"),
    ("", "H", "#", "HH"), ("", "H", "", ""),
    // I
    (" ", "IN", "", "IH N"), (" ", "I", " ", "AY"), ("", "IN", "D", "AY N"), ("", "IER", "", "IY ER"), ("#:R", "IED", "", "IY D"),
    ("", "IED", " ", "AY D"), ("", "IEN", "", "IY EH N"), ("", "IE", "T", "AY EH"), (" :", "I", "%", "AY"), ("", "I", "%", "IY"),
    ("", "IE", "", "IY"), ("", "I", "^+:#", "IH"), ("", "IR", "#", "AY R"), ("", "IZ", "%", "AY Z"), ("", "IS", "%", "AY Z"),
    ("", "I", "D%", "AY"), ("+^", "I", "^+", "IH"), ("", "I", "T%", "AY"), ("#^:", "I", "^+", "IH"), ("", "I", "^+", "AY"),
    ("", "IR", "", "ER"), ("", "IGH", "", "AY"), ("", "ILD", "", "AY L D"), ("", "IGN", " ", "AY N"), ("", "IGN", "^", "AY N"),
    ("", "IGN", "%", "AY N"), ("", "IQUE", "", "IY K"), ("", "I", "", "IH"),
    // J K
    ("", "J", "", "JH"), (" ", "K", "N", ""), ("", "KK", "", "K"), ("", "K", "", "K"),
    // L
    ("", "LO", "C#", "L OW"), ("L", "L", "", ""), ("#^:", "L", "%", "AH L"), ("", "LEAD", "", "L IY D"), ("", "L", "", "L"),
    // M
    ("", "MOV", "", "M UW V"), ("", "MM", "", "M"), ("", "M", "", "M"),
    // N
    ("E", "NG", "+", "N JH"), ("", "NG", "R", "NG G"), ("", "NG", "#", "NG G"), ("", "NGL", "%", "NG G AH L"), ("", "NG", "", "NG"),
    ("", "NK", "", "NG K"), (" ", "NOW", " ", "N AW"), ("", "NN", "", "N"), ("", "N", "", "N"),
    // O
    ("", "OF", " ", "AH V"), ("", "OROUGH", "", "ER OW"), ("#:", "OR", " ", "ER"), ("#:", "ORS", " ", "ER Z"), ("", "OR", "", "AO R"),
    (" ", "ONE", "", "W AH N"), ("", "OW", "", "OW"), (" ", "OVER", "", "OW V ER"), ("", "OV", "", "AH V"), ("", "O", "^%", "OW"),
    ("", "O", "^EN", "OW"), ("", "O", "^I#", "OW"), ("", "OL", "D", "OW L"), ("", "OUGHT", "", "AO T"), ("", "OUGH", "", "AH F"),
    (" ", "OU", "", "AW"), ("H", "OU", "S#", "AW"), ("", "OUS", "", "AH S"), ("", "OUR", "", "AO R"), ("", "OULD", "", "UH D"),
    ("^", "OU", "^L", "AH"), ("", "OUP", "", "UW P"), ("", "OU", "", "AW"), ("", "OY", "", "OY"), ("", "OING", "", "OW IH NG"),
    ("", "OI", "", "OY"), ("", "OOR", "", "AO R"), ("", "OOK", "", "UH K"), ("", "OOD", "", "UH D"), ("", "OO", "", "UW"),
    ("", "O", "E", "OW"), ("", "O", " ", "OW"), ("", "OA", "", "OW"), (" ", "ONLY", "", "OW N L IY"), (" ", "ONCE", "", "W AH N S"),
    ("", "ON'T", "", "OW N T"), ("C", "O", "N", "AA"), ("", "O", "NG", "AO"), (" :^", "O", "N", "AH"), ("I", "ON", "", "AH N"),
    ("#:", "ON", " ", "AH N"), ("#^", "ON", "", "AH N"), ("", "O", "ST ", "OW"), ("", "OF", "^", "AO F"),
    ("", "OTHER", "", "AH DH ER"), ("", "OSS", " ", "AO S"), ("#^:", "OM", "", "AH M"), ("", "O", "", "AA"),
    // P
    ("", "PH", "", "F"), ("", "PEOP", "", "P IY P"), ("", "POW", "", "P AW"), ("", "PUT", " ", "P UH T"), (" ", "PS", "", "S"),
    (" ", "PN", "", "N"), ("", "PP", "", "P"), ("", "P", "", "P"),
    // Q R
    ("", "QUAR", "", "K W AO R"), ("", "QU", "", "K W"), ("", "Q", "", "K"),
    (" ", "RE", "^#", "R IY"), ("", "RH", "", "R"), ("", "RR", "", "R"), ("", "R", "", "R"),
    // S
    ("", "SH", "", "SH"), ("#", "SION", "", "ZH AH N"), ("", "SOME", "", "S AH M"), ("#", "SUR", "#", "ZH ER"),
    ("", "SUR", "#", "SH ER"), ("#", "SU", "#", "ZH UW"), ("#", "SSU", "#", "SH UW"), ("#", "SED", " ", "Z D"),
    ("#", "S", "#", "Z"), ("", "SAID", "", "S EH D"), ("^", "SION", "", "SH AH N"), ("", "S", "S", ""), (".", "S", " ", "Z"),
    ("#:.E", "S", " ", "Z"), ("#^:##", "S", " ", "Z"), ("#^:#", "S", " ", "S"), ("U", "S", " ", "S"), (" :#", "S", " ", "Z"),
    (" ", "SCH", "", "S K"), ("", "S", "C+", ""), ("#", "SM", "", "Z M"), ("#", "SN", "'", "Z AH N"), ("", "S", "", "S"),
    // T
    (" ", "THE", " ", "DH AH"), ("", "TO", " ", "T UW"), ("", "THAT", " ", "DH AE T"), (" ", "THIS", " ", "DH IH S"),
    (" ", "THEY", "", "DH EY"), (" ", "THERE", "", "DH EH R"), ("", "THER", "", "DH ER"), ("", "THEIR", "", "DH EH R"),
    (" ", "THAN", " ", "DH AE N"), (" ", "THEM", " ", "DH EH M"), ("", "THESE", " ", "DH IY Z"), (" ", "THEN", "", "DH EH N"),
    ("", "THROUGH", "", "TH R UW"), ("", "THOSE", "", "DH OW Z"), ("", "THOUGH", " ", "DH OW"), (" ", "THUS", "", "DH AH S"),
    ("", "TH", "", "TH"), ("#:", "TED", " ", "T IH D"), ("S", "TI", "#N", "CH"), ("", "TI", "O", "SH"), ("", "TI", "A", "SH"),
    ("", "TIEN", "", "SH AH N"), ("", "TUR", "#", "CH ER"), ("", "TU", "A", "CH UW"), (" ", "TWO", "", "T UW"),
    ("", "TCH", "", "CH"), ("", "TT", "", "T"), ("", "T", "", "T"),
    // U
    (" ", "UN", "I", "Y UW N"), (" ", "UN", "", "AH N"), (" ", "UPON", "", "AH P AO N"), ("@", "UR", "#", "UH R"),
    ("", "UR", "#", "Y UH R"), ("", "UR", "", "ER"), ("", "U", "^ ", "AH"), ("", "U", "^^", "AH"), ("", "UY", "", "AY"),
    (" G", "U", "#", ""), ("G", "U", "%", ""), ("G", "U", "#", "W"), ("#N", "U", "", "Y UW"), ("@", "U", "", "UW"),
    ("", "U", "", "Y UW"),
    // V W X
    ("", "VIEW", "", "V Y UW"), ("", "VV", "", "V"), ("", "V", "", "V"),
    (" ", "WERE", "", "W ER"), ("", "WA", "S", "W AA"), ("", "WA", "T", "W AA"), ("", "WHERE", "", "W EH R"),
    ("", "WHAT", "", "W AH T"), ("", "WHOL", "", "HH OW L"), ("", "WHO", "", "HH UW"), ("", "WH", "", "W"),
    ("", "WAR", "", "W AO R"), ("", "WOR", "^", "W ER"), ("", "WR", "", "R"), ("", "W", "", "W"),
    (" ", "X", "", "Z"), ("", "X", "", "K S"),
    // Y Z
    ("", "YOUNG", "", "Y AH NG"), (" ", "YOU", "", "Y UW"), (" ", "YES", "", "Y EH S"), (" ", "Y", "", "Y"),
    ("#^:", "Y", " ", "IY"), ("#^:", "Y", "I", "IY"), (" :", "Y", " ", "AY"), (" :", "Y", "#", "AY"),
    (" :", "Y", "^+:#", "IH"), (" :", "Y", "^#", "AY"), ("", "Y", "", "IH"),
    ("", "ZZ", "", "Z"), ("", "Z", "", "Z"),
    // Anything else, such as an apostrophe, is silent.
];

fn is_vowel(c: u8) -> bool {
    matches!(c, b'A' | b'E' | b'I' | b'O' | b'U')
}

fn is_consonant(c: u8) -> bool {
    c.is_ascii_uppercase() && !is_vowel(c)
}

/// Whether the letters before `end` (exclusive) match `context`, read from its end backwards.
fn left_matches(word: &[u8], end: usize, context: &str) -> bool {
    let mut i = end as isize - 1;
    let at = |i: isize| if i < 0 { b' ' } else { word[i as usize] };
    for &symbol in context.as_bytes().iter().rev() {
        match symbol {
            b' ' => {
                if at(i).is_ascii_uppercase() || at(i) == b'\'' {
                    return false;
                }
                i -= 1;
            }
            b'#' => {
                if !is_vowel(at(i)) {
                    return false;
                }
                while is_vowel(at(i)) {
                    i -= 1;
                }
            }
            b':' => {
                while is_consonant(at(i)) {
                    i -= 1;
                }
            }
            b'^' => {
                if !is_consonant(at(i)) {
                    return false;
                }
                i -= 1;
            }
            b'.' => {
                if !matches!(at(i), b'B' | b'D' | b'G' | b'J' | b'L' | b'M' | b'N' | b'R' | b'V' | b'W' | b'Z') {
                    return false;
                }
                i -= 1;
            }
            b'+' => {
                if !matches!(at(i), b'E' | b'I' | b'Y') {
                    return false;
                }
                i -= 1;
            }
            b'&' => {
                if matches!(at(i), b'S' | b'C' | b'G' | b'Z' | b'X' | b'J') {
                    i -= 1;
                } else if at(i) == b'H' && matches!(at(i - 1), b'C' | b'S') {
                    i -= 2;
                } else {
                    return false;
                }
            }
            b'@' => {
                if matches!(at(i), b'T' | b'S' | b'R' | b'D' | b'L' | b'Z' | b'N' | b'J') {
                    i -= 1;
                } else if at(i) == b'H' && matches!(at(i - 1), b'T' | b'C' | b'S') {
                    i -= 2;
                } else {
                    return false;
                }
            }
            letter => {
                if at(i) != letter {
                    return false;
                }
                i -= 1;
            }
        }
    }
    true
}

/// Whether the letters from `start` match `context`, read forwards.
fn right_matches(word: &[u8], start: usize, context: &str) -> bool {
    let mut i = start;
    let at = |i: usize| word.get(i).copied().unwrap_or(b' ');
    for &symbol in context.as_bytes() {
        match symbol {
            b' ' => {
                if at(i).is_ascii_uppercase() || at(i) == b'\'' {
                    return false;
                }
                i += 1;
            }
            b'#' => {
                if !is_vowel(at(i)) {
                    return false;
                }
                while is_vowel(at(i)) {
                    i += 1;
                }
            }
            b':' => {
                while is_consonant(at(i)) {
                    i += 1;
                }
            }
            b'^' => {
                if !is_consonant(at(i)) {
                    return false;
                }
                i += 1;
            }
            b'.' => {
                if !matches!(at(i), b'B' | b'D' | b'G' | b'J' | b'L' | b'M' | b'N' | b'R' | b'V' | b'W' | b'Z') {
                    return false;
                }
                i += 1;
            }
            b'+' => {
                if !matches!(at(i), b'E' | b'I' | b'Y') {
                    return false;
                }
                i += 1;
            }
            b'%' => {
                let rest = &word[i.min(word.len())..];
                let suffix = ["ING", "ELY", "ER", "ES", "ED", "E"].iter().find(|s| rest.starts_with(s.as_bytes()));
                match suffix {
                    Some(s) => i += s.len(),
                    None => return false,
                }
            }
            b'&' => {
                if matches!(at(i), b'S' | b'C' | b'G' | b'Z' | b'X' | b'J') {
                    i += 1;
                } else {
                    return false;
                }
            }
            b'@' => {
                if matches!(at(i), b'T' | b'S' | b'R' | b'D' | b'L' | b'Z' | b'N' | b'J') {
                    i += 1;
                } else {
                    return false;
                }
            }
            letter => {
                if at(i) != letter {
                    return false;
                }
                i += 1;
            }
        }
    }
    true
}

/// ARPAbet symbols to sounds, all unstressed for now.
fn sounds_of(arpabet: &'static str, out: &mut Vec<(&'static str, u8)>) {
    for symbol in arpabet.split(' ').filter(|s| !s.is_empty()) {
        out.push((symbol, 0));
    }
}

/// Endings that take the stress on the syllable before them, as in "nation" and "music", or two before, as in
/// "ability".
const STRESS_BEFORE: [&str; 12] = ["TION", "SION", "CIAN", "CIOUS", "TIOUS", "IAL", "IAN", "IC", "ICS", "IENT", "IENCE", "EOUS"];
const STRESS_TWO_BEFORE: [&str; 4] = ["ITY", "ICAL", "OLOGY", "OGRAPHY"];
/// Endings that are stressed themselves, as in "employee" and "cassette".
const STRESSED_ENDINGS: [&str; 6] = ["EE", "EER", "ESE", "ETTE", "ESQUE", "OON"];
/// Prefixes that are rarely stressed in two-syllable words, as in "begin" and "return".
const UNSTRESSED_PREFIXES: [&str; 12] = ["A", "BE", "DE", "RE", "PRE", "PRO", "CON", "COM", "DIS", "EX", "MIS", "UN"];

/// Picks the stressed vowel: from the word's ending if it has a telling one, after a prefix in two-syllable words,
/// or else on the first syllable.
fn stress(word: &str, sounds: &mut [(&'static str, u8)]) {
    let vowels: Vec<usize> = (0..sounds.len()).filter(|&i| is_arpabet_vowel(sounds[i].0)).collect();
    if vowels.is_empty() {
        return;
    }
    let count = vowels.len();
    let pick = if STRESSED_ENDINGS.iter().any(|e| word.ends_with(e)) {
        count - 1
    } else if STRESS_BEFORE.iter().any(|e| word.ends_with(e)) && count >= 2 {
        // The ending has one vowel, so the stressed one is just before it.
        count - 2
    } else if STRESS_TWO_BEFORE.iter().any(|e| word.ends_with(e)) && count >= 3 {
        let ending_vowels = if word.ends_with("ITY") || word.ends_with("ICAL") { 2 } else { 3 };
        count.saturating_sub(ending_vowels + 1)
    } else if count == 2 && UNSTRESSED_PREFIXES.iter().any(|p| word.len() > p.len() + 2 && word.starts_with(p)) {
        1
    } else {
        0
    };
    sounds[vowels[pick.min(count - 1)]].1 = 1;
}

fn is_arpabet_vowel(symbol: &str) -> bool {
    matches!(symbol.as_bytes()[0], b'A' | b'E' | b'I' | b'O' | b'U')
}

/// ARPAbet symbols with stress numbers to sounds. Unstressed AH and ER become /ə/ and /ɚ/.
pub fn to_sounds(arpabet: &[(&str, u8)]) -> Vec<Sound> {
    arpabet
        .iter()
        .filter_map(|&(symbol, stress)| {
            let a = ARPABET.iter().find(|a| a.symbol == symbol)?;
            let ipa = if stress == 0 { a.unstressed } else { a.ipa };
            Some((ENGLISH.find(ipa)?, stress))
        })
        .collect()
}

/// Says a word from its spelling. Letters other than A to Z and apostrophes are ignored.
pub fn pronounce(word: &str) -> Vec<Sound> {
    let upper: Vec<u8> = word.bytes().map(|b| b.to_ascii_uppercase()).filter(|b| b.is_ascii_uppercase() || *b == b'\'').collect();
    let mut arpabet = Vec::new();
    let mut i = 0;
    while i < upper.len() {
        let c = upper[i];
        let rule = RULES.iter().find(|(left, middle, right, _)| {
            middle.as_bytes()[0] == c
                && upper[i..].starts_with(middle.as_bytes())
                && left_matches(&upper, i, left)
                && right_matches(&upper, i + middle.len(), right)
        });
        match rule {
            Some(&(_, middle, _, out)) => {
                sounds_of(out, &mut arpabet);
                i += middle.len();
            }
            None => i += 1,
        }
    }
    let word = String::from_utf8_lossy(&upper).into_owned();
    stress(&word, &mut arpabet);
    to_sounds(&arpabet)
}

/// Each letter's name, for spelling words out.
const LETTER_NAMES: [&str; 26] = [
    "EY1", "B IY1", "S IY1", "D IY1", "IY1", "EH1 F", "JH IY1", "EY1 CH", "AY1", "JH EY1", "K EY1", "EH1 L", "EH1 M", "EH1 N",
    "OW1", "P IY1", "K Y UW1", "AA1 R", "EH1 S", "T IY1", "Y UW1", "V IY1", "D AH1 B AH0 L Y UW0", "EH1 K S", "W AY1", "Z IY1",
];

/// The name of a letter, as said when spelling: "B" is /ˈbi/.
pub fn letter_name(letter: char) -> Vec<Sound> {
    let Some(index) = (letter.to_ascii_uppercase() as u32).checked_sub('A' as u32).filter(|&i| i < 26) else { return Vec::new() };
    let arpabet: Vec<(&str, u8)> = LETTER_NAMES[index as usize]
        .split(' ')
        .map(|s| match s.as_bytes()[s.len() - 1] {
            d @ b'0'..=b'2' => (&s[..s.len() - 1], d - b'0'),
            _ => (s, 0),
        })
        .collect();
    to_sounds(&arpabet)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn said(word: &str) -> String {
        pronounce(word).iter().map(|(p, s)| if p.is_vowel() { format!("{}{s}", p.ipa) } else { p.ipa.to_string() }).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn common_spellings() {
        assert_eq!(said("cat"), "k æ1 t");
        assert_eq!(said("make"), "m eɪ1 k");
        assert_eq!(said("night"), "n aɪ1 t");
        assert_eq!(said("phone"), "f oʊ1 n");
        assert_eq!(said("ship"), "ʃ ɪ1 p");
        assert_eq!(said("chat"), "tʃ æ1 t");
        assert_eq!(said("thin"), "θ ɪ1 n");
        assert_eq!(said("knee"), "n i1");
        assert_eq!(said("bloop"), "b l u1 p");
    }

    #[test]
    fn endings_move_the_stress() {
        assert_eq!(said("glorbation"), "ɡ l ɔ0 ɹ b eɪ1 ʃ ə0 n");
        assert_eq!(said("redo").contains("oʊ1") || said("redo").contains("ɛ1") || said("redo").contains("i1"), true);
    }

    #[test]
    fn every_word_gets_one_stress() {
        for word in ["blorptastic", "zib", "frumious", "snicker", "vorpal", "outgrabe", "mimsy", "tulgey", "a"] {
            let stressed = pronounce(word).iter().filter(|(p, s)| p.is_vowel() && *s == 1).count();
            assert_eq!(stressed, 1, "{word}: {}", said(word));
        }
    }

    #[test]
    fn letter_names() {
        let name = |c| letter_name(c).iter().map(|(p, _)| p.ipa).collect::<String>();
        assert_eq!(name('b'), "bi");
        assert_eq!(name('W'), "dʌbəlju");
        assert_eq!(name('h'), "eɪtʃ");
        assert!(letter_name('1').is_empty());
    }
}
