//! Reads English text aloud: from words to sounds, then intonation.
//!
//! 1. `text` turns the text into words and pauses (numbers, abbreviations, and symbols become words).
//! 2. Each word is said, in this order of preference: spelled out (acronyms), a weak form for a small function word
//!    ("of" as /əv/), the pronouncing dictionary (choosing between a heteronym's pronunciations from the words
//!    around it), a dictionary word with an ending or prefix added ("blogging" from "blog"), two dictionary words
//!    joined ("dreamboat"), and last, the letter-to-sound rules.
//! 3. Long stretches without punctuation get a short breath before a joining word such as "and".
//! 4. `prosody` sets each vowel's pitch.

use super::letters::{self, to_sounds};
use super::prosody;
use super::text::{Piece, normalize};
use crate::dictionary::{Dictionary, Sound};
use crate::language::Reading;
use crate::phonemes::{MAX_TOKENS, ParseError, Token};

/// A word as it will be said.
#[derive(Clone, Debug)]
pub struct Said {
    pub text: String,
    pub sounds: Vec<Sound>,
    /// Each sound's pitch, filled in by `prosody` for vowels.
    pub tones: Vec<Option<(f32, f32)>>,
    pub offset: usize,
    pub len: usize,
    /// A small grammatical word ("the", "of", "is") that is not accented.
    pub function: bool,
    /// One letter of a spelled word.
    pub spelled: bool,
}

#[derive(Clone, Debug)]
pub enum Spoken {
    Word(Said),
    Break { mark: char, offset: usize },
}

/// Reads ARPAbet with stress numbers, such as `DH AH0`, as sounds.
fn arpabet(text: &str) -> Vec<Sound> {
    let symbols: Vec<(&str, u8)> = text
        .split(' ')
        .map(|s| match s.as_bytes()[s.len() - 1] {
            d @ b'0'..=b'2' => (&s[..s.len() - 1], d - b'0'),
            _ => (s, 0),
        })
        .collect();
    to_sounds(&symbols)
}

/// How a function word behaves at the end of a phrase.
#[derive(Clone, Copy, PartialEq)]
enum Class {
    /// Articles and pronouns stay weak: "I saw it."
    Weak,
    /// Prepositions and auxiliaries take their full form: "Who is it for?", "I think I can."
    Strong,
}

/// Small grammatical words and their weak (unstressed) forms.
#[rustfmt::skip]
const FUNCTION_WORDS: [(&str, &str, Class); 61] = [
    ("a", "AH0", Class::Weak), ("an", "AH0 N", Class::Weak), ("the", "DH AH0", Class::Weak),
    ("of", "AH0 V", Class::Strong), ("to", "T AH0", Class::Strong), ("for", "F ER0", Class::Strong),
    ("from", "F R AH0 M", Class::Strong), ("at", "AH0 T", Class::Strong), ("in", "IH0 N", Class::Strong),
    ("on", "AA0 N", Class::Strong), ("by", "B AY0", Class::Strong), ("with", "W IH0 DH", Class::Strong),
    ("as", "AH0 Z", Class::Strong), ("into", "IH0 N T UW0", Class::Strong), ("onto", "AA0 N T UW0", Class::Strong),
    ("i", "AY0", Class::Weak), ("me", "M IY0", Class::Weak), ("you", "Y UW0", Class::Weak), ("he", "HH IY0", Class::Weak),
    ("she", "SH IY0", Class::Weak), ("it", "IH0 T", Class::Weak), ("we", "W IY0", Class::Weak), ("they", "DH EY0", Class::Weak),
    ("him", "HH IH0 M", Class::Weak), ("her", "HH ER0", Class::Weak), ("us", "AH0 S", Class::Weak), ("them", "DH AH0 M", Class::Weak),
    ("my", "M AY0", Class::Weak), ("your", "Y ER0", Class::Weak), ("his", "HH IH0 Z", Class::Weak), ("its", "IH0 T S", Class::Weak),
    ("our", "AA0 R", Class::Weak), ("their", "DH EH0 R", Class::Weak),
    ("am", "AH0 M", Class::Strong), ("is", "IH0 Z", Class::Strong), ("are", "ER0", Class::Strong), ("was", "W AH0 Z", Class::Strong),
    ("were", "W ER0", Class::Strong), ("be", "B IY0", Class::Strong), ("been", "B IH0 N", Class::Strong),
    ("have", "HH AH0 V", Class::Strong), ("has", "HH AH0 Z", Class::Strong), ("had", "HH AH0 D", Class::Strong),
    ("do", "D UW0", Class::Strong), ("does", "D AH0 Z", Class::Strong), ("did", "D IH0 D", Class::Strong),
    ("can", "K AH0 N", Class::Strong), ("could", "K UH0 D", Class::Strong), ("will", "W IH0 L", Class::Strong),
    ("would", "W UH0 D", Class::Strong), ("shall", "SH AH0 L", Class::Strong), ("should", "SH UH0 D", Class::Strong),
    ("must", "M AH0 S T", Class::Strong), ("and", "AH0 N D", Class::Weak), ("but", "B AH0 T", Class::Weak),
    ("or", "ER0", Class::Weak), ("nor", "N ER0", Class::Weak), ("than", "DH AH0 N", Class::Weak), ("if", "IH0 F", Class::Weak),
    ("that", "DH AH0 T", Class::Weak), ("there", "DH ER0", Class::Weak),
];

const DETERMINERS: [&str; 22] = [
    "a", "an", "the", "this", "that", "these", "those", "my", "your", "his", "her", "its", "our", "their", "no", "any", "every",
    "each", "some", "another", "what", "which",
];
const SUBJECTS: [&str; 9] = ["i", "you", "we", "they", "he", "she", "it", "who", "people"];
const BEFORE_VERBS: [&str; 26] = [
    "to", "will", "would", "can", "could", "should", "might", "must", "shall", "may", "do", "does", "did", "don't", "didn't",
    "won't", "can't", "cannot", "let's", "please", "not", "never", "always", "also", "often", "usually",
];
const BE: [&str; 8] = ["am", "is", "are", "was", "were", "be", "been", "being"];
const HAVE: [&str; 4] = ["have", "has", "had", "having"];
const PREPOSITIONS: [&str; 10] = ["of", "in", "on", "for", "with", "at", "from", "about", "by", "into"];

/// Words said as a noun (or adjective) with early stress, and as a verb with later stress: "a REcord", "to reCORD".
const STRESS_SHIFTS: [&str; 52] = [
    "record", "present", "object", "project", "permit", "produce", "conduct", "contract", "contest", "convert", "convict",
    "desert", "increase", "decrease", "insult", "progress", "rebel", "refuse", "subject", "suspect", "conflict", "combat",
    "compound", "digest", "discount", "export", "import", "extract", "impact", "insert", "reject", "survey", "torment",
    "transport", "upset", "protest", "perfect", "content", "console", "entrance", "rewrite", "recall", "update", "transfer",
    "escort", "contrast", "exploit", "frequent", "implant", "refund", "segment", "upgrade",
];

/// How to choose between a heteronym's pronunciations in the dictionary.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Pick {
    /// The one whose stressed vowel is this sound.
    Vowel(&'static str),
    /// The one ending in this sound.
    Ends(&'static str),
    /// Stress on the first vowel (nouns), or later (verbs).
    EarlyStress,
    LateStress,
}

fn is_in(list: &[&str], word: Option<&str>) -> bool {
    word.is_some_and(|w| list.contains(&w))
}

/// Chooses a heteronym's pronunciation from the words before and after it. `None` keeps the dictionary's first.
fn heteronym(word: &str, before2: Option<&str>, before: Option<&str>, after: Option<&str>, after2: Option<&str>) -> Option<Pick> {
    let verb_context = is_in(&BEFORE_VERBS, before) || is_in(&SUBJECTS, before);
    // A determiner just before, or before one other word ("a new record").
    let noun_context = is_in(&DETERMINERS, before) || is_in(&PREPOSITIONS, before) || (is_in(&DETERMINERS, before2) && !verb_context);
    Some(match word {
        "read" => {
            let past_after = ["yesterday", "ago", "earlier", "last"];
            if is_in(&HAVE, before) || is_in(&BE, before) || is_in(&["already", "just", "once", "recently"], before) || is_in(&past_after, after) || is_in(&past_after, after2) {
                Pick::Vowel("ɛ")
            } else {
                Pick::Vowel("i")
            }
        }
        "live" => {
            let adjective_after = ["music", "show", "shows", "stream", "streams", "streaming", "event", "events", "broadcast", "performance", "audience", "band", "album", "action", "wire", "bait", "coverage", "tv", "radio", "chat", "demo", "feed", "version", "recording", "concert", "animals"];
            if is_in(&adjective_after, after) || is_in(&BE, before) || is_in(&DETERMINERS, before) || is_in(&["go", "goes", "went", "going"], before) {
                Pick::Vowel("aɪ")
            } else {
                Pick::Vowel("ɪ")
            }
        }
        "lives" => if noun_context || is_in(&["save", "saved", "saves", "saving", "many", "two", "three", "nine", "whose", "lost", "our", "their", "your"], before) { Pick::Vowel("aɪ") } else { Pick::Vowel("ɪ") },
        "lead" => {
            let metal_after = ["pipe", "pipes", "paint", "poisoning", "pencil", "pencils", "weight", "weights", "balloon", "shot", "crystal", "acid", "free"];
            if is_in(&metal_after, after) || is_in(&["of", "with", "unleaded", "red", "white"], before) { Pick::Vowel("ɛ") } else { Pick::Vowel("i") }
        }
        "tear" | "tears" => {
            let verb_after = ["up", "down", "apart", "off", "open", "through", "into", "out", "it", "them", "the", "away"];
            if verb_context || is_in(&verb_after, after) { Pick::Vowel("ɛ") } else { Pick::Vowel("ɪ") }
        }
        "wind" | "winds" => {
            if verb_context || is_in(&["up", "down", "around", "back", "it", "the"], after) { Pick::Vowel("aɪ") } else { Pick::Vowel("ɪ") }
        }
        "wound" => {
            if is_in(&["up", "down", "around", "through", "back", "its", "his", "her", "the"], after) && !noun_context { Pick::Vowel("aʊ") } else { Pick::Vowel("u") }
        }
        "bow" | "bows" => {
            if is_in(&["tie", "ties", "string", "strings"], after) || (after == Some("and") && is_in(&["arrow", "arrows"], after2)) || is_in(&["violin", "cello", "hair"], before) {
                Pick::Vowel("oʊ")
            } else {
                Pick::Vowel("aʊ")
            }
        }
        "close" => {
            let near_after = ["to", "by", "call", "calls", "friend", "friends", "enough", "together", "relative", "relatives", "attention", "contact", "second", "behind"];
            let near_before = ["very", "so", "too", "a", "is", "are", "was", "were", "be", "been", "get", "got", "come", "came", "stay", "stayed", "quite", "really", "how", "that", "getting", "up"];
            if is_in(&near_after, after) || is_in(&near_before, before) { Pick::Ends("s") } else { Pick::Ends("z") }
        }
        "use" | "excuse" | "abuse" | "refuse" if word != "refuse" || noun_context => {
            if noun_context || is_in(&["no", "much", "good", "out", "fair", "its"], before) { Pick::Ends("s") } else { Pick::Ends("z") }
        }
        w if STRESS_SHIFTS.contains(&w) => {
            if verb_context { Pick::LateStress } else if noun_context { Pick::EarlyStress } else { return None }
        }
        _ => return None,
    })
}

fn choose(pronunciations: &[Vec<Sound>], pick: Pick) -> Option<&Vec<Sound>> {
    pronunciations.iter().find(|sounds| {
        let primary = sounds.iter().position(|(p, s)| p.is_vowel() && *s == 1);
        let first_vowel = sounds.iter().position(|(p, _)| p.is_vowel());
        match pick {
            Pick::Vowel(ipa) => primary.is_some_and(|i| sounds[i].0.ipa == ipa),
            Pick::Ends(ipa) => sounds.last().is_some_and(|(p, _)| p.ipa == ipa),
            Pick::EarlyStress => primary.is_some() && primary == first_vowel,
            Pick::LateStress => primary.is_some() && primary != first_vowel,
        }
    })
}

/// The sounds that end a word decide how "-s" and "-ed" sound.
fn ends_with(sounds: &[Sound], ipas: &[&str]) -> bool {
    sounds.last().is_some_and(|(p, _)| ipas.contains(&p.ipa))
}

/// Adds "-s": /ɪz/ after a hiss, /s/ after other voiceless sounds, else /z/.
fn add_s(mut sounds: Vec<Sound>) -> Vec<Sound> {
    let ending = if ends_with(&sounds, &["s", "z", "ʃ", "ʒ", "tʃ", "dʒ"]) {
        "IH0 Z"
    } else if ends_with(&sounds, &["p", "t", "k", "f", "θ"]) {
        "S"
    } else {
        "Z"
    };
    sounds.extend(arpabet(ending));
    sounds
}

/// Adds "-ed": /ɪd/ after t and d, /t/ after other voiceless sounds, else /d/.
fn add_ed(mut sounds: Vec<Sound>) -> Vec<Sound> {
    let ending = if ends_with(&sounds, &["t", "d"]) {
        "IH0 D"
    } else if ends_with(&sounds, &["p", "k", "f", "θ", "s", "ʃ", "tʃ"]) {
        "T"
    } else {
        "D"
    };
    sounds.extend(arpabet(ending));
    sounds
}

/// The stem a word with this ending may have come from: as is, with a silent e put back, with a doubled consonant
/// undone, or with y put back for i.
fn stems(stem: &str) -> Vec<String> {
    let mut stems = vec![stem.to_string(), format!("{stem}e")];
    let bytes = stem.as_bytes();
    if bytes.len() >= 3 && bytes[bytes.len() - 1] == bytes[bytes.len() - 2] {
        stems.push(stem[..stem.len() - 1].to_string());
    }
    if let Some(s) = stem.strip_suffix('i') {
        stems.push(format!("{s}y"));
    }
    stems
}

/// Endings added to a dictionary word, with their sounds. `-s` and `-ed` change with the word's last sound.
const SUFFIXES: [(&str, &str); 16] = [
    ("'s", "S"), ("s'", "S"), ("es", "S"), ("s", "S"), ("ed", "ED"), ("ing", "IH0 NG"), ("ers", "ER0 Z"), ("er", "ER0"),
    ("est", "AH0 S T"), ("ly", "L IY0"), ("ness", "N AH0 S"), ("less", "L AH0 S"), ("ful", "F AH0 L"), ("ments", "M AH0 N T S"),
    ("ment", "M AH0 N T"), ("able", "AH0 B AH0 L"),
];

/// Prefixes added to a dictionary word, which keeps its own stress.
const PREFIXES: [(&str, &str); 15] = [
    ("un", "AH2 N"), ("re", "R IY2"), ("non", "N AA2 N"), ("pre", "P R IY2"), ("over", "OW2 V ER0"), ("under", "AH2 N D ER0"),
    ("out", "AW2 T"), ("mis", "M IH2 S"), ("dis", "D IH2 S"), ("anti", "AE2 N T IY0"), ("super", "S UW2 P ER0"),
    ("semi", "S EH2 M IY0"), ("multi", "M AH2 L T IY0"), ("inter", "IH2 N T ER0"), ("sub", "S AH2 B"),
];

/// Lowers a word's primary stress to secondary, for the second part of a compound.
fn demote(sounds: Vec<Sound>) -> Vec<Sound> {
    sounds.into_iter().map(|(p, s)| (p, if s == 1 { 2 } else { s })).collect()
}

/// Says a word the dictionary doesn't have from parts it does: an ending, a prefix, or two words joined.
fn derive(word: &str, dictionary: &Dictionary, depth: u8) -> Option<Vec<Sound>> {
    if depth == 0 || word.len() < 4 {
        return None;
    }
    let known = |w: &str| -> Option<Vec<Sound>> {
        dictionary.lookup(w).into_iter().next().or_else(|| derive(w, dictionary, depth - 1))
    };
    for (suffix, ending) in SUFFIXES {
        let Some(stem) = word.strip_suffix(suffix).filter(|s| s.len() >= 2) else { continue };
        let candidates = if matches!(suffix, "'s" | "s'" | "s") { vec![stem.to_string()] } else { stems(stem) };
        for candidate in candidates {
            if let Some(sounds) = known(&candidate) {
                return Some(match ending {
                    "S" => add_s(sounds),
                    "ED" => add_ed(sounds),
                    _ => {
                        let mut sounds = sounds;
                        sounds.extend(arpabet(ending));
                        sounds
                    }
                });
            }
        }
    }
    for (prefix, start) in PREFIXES {
        if let Some(rest) = word.strip_prefix(prefix).filter(|r| r.len() >= 3)
            && let Some(sounds) = known(rest)
        {
            let mut all = arpabet(start);
            all.extend(sounds);
            return Some(all);
        }
    }
    // Two words joined, the longest first part first: "dreamboat". An s between them goes with the second word, as
    // in "doomscroll", when both splits are words.
    let both = |split: usize| {
        let (first, second) = word.split_at(split);
        dictionary.contains(first) && dictionary.contains(second)
    };
    for split in (3..=word.len().saturating_sub(3)).rev() {
        if !word.is_char_boundary(split) || (word.as_bytes()[split - 1] == b's' && both(split - 1)) {
            continue;
        }
        let (first, second) = word.split_at(split);
        if let (Some(a), Some(b)) = (dictionary.lookup(first).into_iter().next(), dictionary.lookup(second).into_iter().next()) {
            let mut all = a;
            all.extend(demote(b));
            return Some(all);
        }
    }
    None
}

/// Short words in capitals that are spelled, though the dictionary has them as words: "US", "IT".
const SPELLED_CAPS: [&str; 8] = ["us", "uk", "eu", "ai", "tv", "pc", "id", "it"];

/// Keeps only the last primary stress, lowering the others to secondary. The dictionary gives some acronyms a primary
/// stress on every letter.
fn one_primary(mut sounds: Vec<Sound>) -> Vec<Sound> {
    if let Some(last) = sounds.iter().rposition(|(p, s)| p.is_vowel() && *s == 1) {
        for sound in &mut sounds[..last] {
            if sound.1 == 1 {
                sound.1 = 2;
            }
        }
    }
    sounds
}

fn has_vowel_letter(word: &str) -> bool {
    word.contains(['a', 'e', 'i', 'o', 'u', 'y'])
}

/// Whether a word in capitals is an acronym to spell: a short one with no vowels or on the list, or one the
/// dictionary doesn't have.
fn spell_caps(word: &str, caps: bool, dictionary: Option<&Dictionary>) -> bool {
    caps && ((word.len() <= 5 && (SPELLED_CAPS.contains(&word) || !has_vowel_letter(word)))
        || (word.len() <= 4 && dictionary.is_some_and(|d| !d.contains(word))))
}

/// The sounds for one word, and whether it is a function word.
fn pronounce(words: &[(String, bool, bool)], i: usize, dictionary: Option<&Dictionary>, phrase_end: bool) -> (Vec<Sound>, bool) {
    let (word, spell) = (&words[i].0, words[i].1);
    let before = i.checked_sub(1).map(|j| words[j].0.as_str());
    let before2 = i.checked_sub(2).map(|j| words[j].0.as_str());
    let after = words.get(i + 1).map(|w| w.0.as_str());
    let after2 = words.get(i + 2).map(|w| w.0.as_str());
    if spell {
        return (word.chars().flat_map(letters::letter_name).collect(), false);
    }
    if let Some(&(_, weak, class)) = FUNCTION_WORDS.iter().find(|f| f.0 == word) {
        // "have", "do", and "that" are only weak as helpers: "I have gone", "do you", "so that he".
        let helper = match word.as_str() {
            "have" | "has" | "had" => after.is_some_and(|w| w.ends_with("ed") || w.ends_with("en") || ["been", "got", "gone", "done", "seen", "made", "said", "come", "become", "known", "had", "to", "not", "never", "just", "already", "read", "heard", "found", "thought", "told", "left", "lost", "put", "won", "bought", "brought", "taught", "kept", "felt", "met", "sent", "spent", "built", "held", "led", "run", "begun", "sung", "swum"].contains(&w)),
            "do" | "does" | "did" => is_in(&SUBJECTS, after) || is_in(&DETERMINERS, after) || is_in(&["not", "n't"], after),
            "that" => is_in(&SUBJECTS, after) || is_in(&DETERMINERS, after),
            "there" => is_in(&BE, after) || is_in(&["'s", "seems", "seem"], after),
            _ => true,
        };
        if helper && !(phrase_end && class == Class::Strong) {
            return (arpabet(weak), true);
        }
    }
    if let Some(dictionary) = dictionary {
        let pronunciations = dictionary.lookup(word);
        if let Some(first) = pronunciations.first() {
            let picked = heteronym(word, before2, before, after, after2).and_then(|pick| choose(&pronunciations, pick));
            return (one_primary(picked.unwrap_or(first).clone()), false);
        }
        if let Some(sounds) = derive(word, dictionary, 2) {
            return (sounds, false);
        }
    }
    (letters::pronounce(word), false)
}

/// Joining words before which a long phrase can take a breath.
const BREATH_BEFORE: [&str; 15] = ["and", "but", "or", "because", "which", "who", "while", "although", "though", "where", "when", "unless", "until", "since", "so"];
/// Words in a phrase before a breath may be added, and words that must follow it.
const BREATH_AFTER_WORDS: usize = 10;
const BREATH_MIN_REST: usize = 3;

/// Reads `text`. The dictionary is optional: without it, every word comes from the letter-to-sound rules.
pub fn read(text: &str, dictionary: Option<&Dictionary>) -> Result<Reading, ParseError> {
    let pieces = normalize(text);
    let words: Vec<(String, bool, bool)> = pieces
        .iter()
        .filter_map(|p| match p {
            Piece::Word(w) => Some((w.text.clone(), w.spell, w.caps)),
            Piece::Break { .. } => None,
        })
        .collect();

    // Say each word.
    let mut spoken = Vec::with_capacity(pieces.len());
    let mut index = 0;
    for (n, piece) in pieces.iter().enumerate() {
        match piece {
            Piece::Word(w) if spell_caps(&w.text, w.caps, dictionary) => {
                // Each letter is a word of its own, so the transcript shows them apart.
                for letter in w.text.chars() {
                    let sounds = letters::letter_name(letter);
                    spoken.push(Spoken::Word(Said {
                        text: letter.to_string(),
                        tones: vec![None; sounds.len()],
                        sounds,
                        offset: w.offset,
                        len: w.len,
                        function: false,
                        spelled: true,
                    }));
                }
                index += 1;
            }
            Piece::Word(w) => {
                let phrase_end = !matches!(pieces.get(n + 1), Some(Piece::Word(_)));
                let (sounds, function) = pronounce(&words, index, dictionary, phrase_end);
                spoken.push(Spoken::Word(Said {
                    text: w.text.clone(),
                    tones: vec![None; sounds.len()],
                    sounds,
                    offset: w.offset,
                    len: w.len,
                    function,
                    spelled: w.spell,
                }));
                index += 1;
            }
            &Piece::Break { mark, offset } => spoken.push(Spoken::Break { mark, offset }),
        }
    }
    adjust_before_vowels(&mut spoken);
    stress_spelled_words(&mut spoken);
    add_breaths(&mut spoken);
    prosody::apply(&mut spoken);

    let mut tokens = Vec::new();
    for item in &spoken {
        match item {
            Spoken::Word(said) => {
                if tokens.len() + said.sounds.len() > MAX_TOKENS {
                    return Err(ParseError { offset: said.offset, len: text.len() - said.offset });
                }
                for (&(phoneme, stress), &tone) in said.sounds.iter().zip(&said.tones) {
                    tokens.push(Token::Phone { phoneme, stress, tone, offset: said.offset, len: said.len });
                }
            }
            &Spoken::Break { mark, offset } => {
                if tokens.len() >= MAX_TOKENS {
                    return Err(ParseError { offset, len: text.len() - offset });
                }
                tokens.push(Token::Pause { mark, offset });
            }
        }
    }
    Ok(Reading { tokens, transcript: transcript(&spoken) })
}

/// "the" is /ði/ and "to" is /tu/ before a vowel: "the apple", "to eat".
fn adjust_before_vowels(spoken: &mut [Spoken]) {
    for i in 0..spoken.len().saturating_sub(1) {
        let next_vowel = match &spoken[i + 1] {
            Spoken::Word(next) => next.sounds.first().is_some_and(|(p, _)| p.is_vowel()),
            _ => false,
        };
        if let Spoken::Word(said) = &mut spoken[i]
            && said.function
            && next_vowel
        {
            let replacement = match said.text.as_str() {
                "the" => "DH IY0",
                "to" => "T UW0",
                _ => continue,
            };
            said.sounds = arpabet(replacement);
            said.tones = vec![None; said.sounds.len()];
        }
    }
}

/// A spelled word stresses its last letter most: "F B I" is /ˌɛf ˌbi ˈaɪ/.
fn stress_spelled_words(spoken: &mut [Spoken]) {
    for i in 0..spoken.len() {
        let next_same = match (&spoken[i], spoken.get(i + 1)) {
            (Spoken::Word(a), Some(Spoken::Word(b))) => a.spelled && b.spelled && a.offset == b.offset,
            _ => false,
        };
        if let Spoken::Word(said) = &mut spoken[i]
            && said.spelled
            && next_same
        {
            for sound in &mut said.sounds {
                if sound.1 == 1 {
                    sound.1 = 2;
                }
            }
        }
    }
}

/// Adds a short pause before a joining word in a long phrase.
fn add_breaths(spoken: &mut Vec<Spoken>) {
    let mut i = 0;
    let mut run = 0;
    while i < spoken.len() {
        match &spoken[i] {
            Spoken::Break { .. } => run = 0,
            Spoken::Word(said) => {
                let rest = spoken[i..].iter().take_while(|s| matches!(s, Spoken::Word(_))).count();
                if run >= BREATH_AFTER_WORDS && rest > BREATH_MIN_REST && BREATH_BEFORE.contains(&said.text.as_str()) {
                    let offset = said.offset;
                    spoken.insert(i, Spoken::Break { mark: ',', offset });
                    run = 0;
                    i += 1;
                }
                run += 1;
            }
        }
        i += 1;
    }
}

/// Whether consonants can start an English syllable together, as "str" and "pl" can.
fn is_onset(consonants: &[&str]) -> bool {
    match consonants {
        [] => true,
        [c] => *c != "ŋ",
        ["s", c] => ["p", "t", "k", "m", "n", "l", "w", "f"].contains(c),
        [a, "ɹ"] => ["p", "b", "t", "d", "k", "ɡ", "f", "θ", "ʃ"].contains(a),
        [a, "l"] => ["p", "b", "k", "ɡ", "f", "s"].contains(a),
        [a, "w"] => ["t", "d", "k", "ɡ", "θ", "s"].contains(a),
        [a, "j"] => ["p", "b", "k", "ɡ", "f", "v", "m", "h"].contains(a),
        ["s", b, "ɹ"] => ["p", "t", "k"].contains(b),
        ["s", "p", "l"] | ["s", "k", "w"] | ["s", "p", "j"] | ["s", "k", "j"] => true,
        _ => false,
    }
}

/// Writes one word in IPA, with stress marks before the stressed syllables.
fn write_word(sounds: &[Sound], out: &mut String) {
    let mut marks = vec![None; sounds.len()];
    let mut previous_vowel: Option<usize> = None;
    for (v, &(p, stress)) in sounds.iter().enumerate() {
        if !p.is_vowel() {
            continue;
        }
        if stress > 0 {
            let start = previous_vowel.map_or(0, |u| u + 1);
            let mut at = v;
            while at > start {
                let consonants: Vec<&str> = sounds[at - 1..v].iter().map(|(p, _)| p.ipa).collect();
                if !is_onset(&consonants) {
                    break;
                }
                at -= 1;
            }
            // The first syllable of a word starts at the word's start.
            if previous_vowel.is_none() {
                at = 0;
            }
            marks[at] = Some(if stress == 1 { 'ˈ' } else { 'ˌ' });
        }
        previous_vowel = Some(v);
    }
    for (sound, mark) in sounds.iter().zip(marks) {
        if let Some(mark) = mark {
            out.push(mark);
        }
        out.push_str(sound.0.ipa);
    }
}

/// The whole reading in IPA, a word at a time, with the pauses' marks.
fn transcript(spoken: &[Spoken]) -> String {
    let mut out = String::new();
    for item in spoken {
        match item {
            Spoken::Word(said) => {
                if !out.is_empty() {
                    out.push(' ');
                }
                write_word(&said.sounds, &mut out);
            }
            Spoken::Break { mark, .. } => out.push(*mark),
        }
    }
    out
}

#[cfg(test)]
pub mod tests {
    use super::*;
    pub use crate::dictionary::english_for_tests as dictionary;

    fn ipa(text: &str) -> String {
        read(text, Some(dictionary())).unwrap().transcript
    }

    #[test]
    fn reads_sentences() {
        assert_eq!(ipa("Hello, world!"), "həˈloʊ, ˈwɝld!");
        assert_eq!(ipa("The cat sat on the mat."), "ðə ˈkæt ˈsæt ɑn ðə ˈmæt.");
    }

    #[test]
    fn function_words_are_weak_except_at_the_end() {
        assert_eq!(ipa("a cup of tea"), "ə ˈkʌp əv ˈti");
        assert_eq!(ipa("What is it for?"), "ˈwʌt ɪz ɪt ˈfɔɹ?");
        assert_eq!(ipa("the apple"), "ði ˈæpəl");
        assert_eq!(ipa("I have cats"), "aɪ ˈhæv ˈkæts");
        assert_eq!(ipa("I have gone"), "aɪ həv ˈɡɔn");
    }

    #[test]
    fn heteronyms_follow_the_words_around_them() {
        assert_eq!(ipa("I will read it"), "aɪ wɪl ˈɹid ɪt");
        assert_eq!(ipa("I have read it"), "aɪ həv ˈɹɛd ɪt");
        assert_eq!(ipa("they live here"), "ðeɪ ˈlɪv ˈhiɹ");
        assert_eq!(ipa("a live show"), "ə ˈlaɪv ˈʃoʊ");
        assert!(ipa("to record it").contains("ɹəˈkɔɹd") || ipa("to record it").contains("ɹɪˈkɔɹd"));
        assert_eq!(ipa("the record"), "ðə ˈɹɛkɚd");
        assert_eq!(ipa("a new record"), "ə ˈnu ˈɹɛkɚd");
        assert_eq!(ipa("what I read yesterday"), "ˈwʌt aɪ ˈɹɛd ˈjɛstɚˌdeɪ");
        assert_eq!(ipa("I read books"), "aɪ ˈɹid ˈbʊks");
        assert_eq!(ipa("close the door"), "ˈkloʊz ðə ˈdɔɹ");
        assert_eq!(ipa("very close"), "ˈvɛɹi ˈkloʊs");
        assert_eq!(ipa("a tear fell"), "ə ˈtɪɹ ˈfɛl");
        assert_eq!(ipa("don't tear it"), "ˈdoʊnt ˈtɛɹ ɪt");
    }

    #[test]
    fn unknown_words_come_from_parts() {
        assert_eq!(ipa("dreamboats"), ipa("dreamboat") + "s");
        assert_eq!(ipa("podcasted"), ipa("podcast") + "ɪd");
        assert_eq!(ipa("teacups"), ipa("teacup") + "s");
        assert_eq!(ipa("unfriended"), "ˌʌnˈfɹɛndɪd");
        assert_eq!(ipa("rewatchable"), "ˌɹiˈwɑtʃəbəl");
        assert_eq!(ipa("doomscrolling"), "ˈdumˌskɹoʊlɪŋ");
        assert_eq!(ipa("zorbled").chars().filter(|&c| c == 'ˈ').count(), 1);
    }

    #[test]
    fn acronyms_are_spelled() {
        assert_eq!(ipa("the FBI"), "ði ˌɛfˌbiˈaɪ");
        assert_eq!(ipa("the US"), "ðə ˌju ˈɛs");
        assert_eq!(ipa("NASA"), "ˈnæsə");
        assert_eq!(ipa("XKCD"), "ˌɛks ˌkeɪ ˌsi ˈdi");
        assert_eq!(ipa("the F.B.I."), "ði ˌɛf ˌbi ˈaɪ.");
    }

    #[test]
    fn numbers_and_symbols() {
        assert_eq!(ipa("$5"), "ˈfaɪv ˈdɑlɚz");
        assert_eq!(ipa("in 1984"), "ɪn ˌnaɪnˈtin ˈeɪti ˈfɔɹ");
    }

    #[test]
    fn long_phrases_take_a_breath() {
        let text = "the quick brown fox jumped over the lazy dog in the park and then it ran home to sleep";
        assert!(ipa(text).contains(", ənd"), "{}", ipa(text));
        assert!(!ipa("cats and dogs").contains(','));
    }

    #[test]
    fn words_keep_their_place_in_the_text() {
        let reading = read("Hi there.", Some(dictionary())).unwrap();
        let places: Vec<(usize, usize)> = reading
            .tokens
            .iter()
            .map(|t| match *t {
                Token::Phone { offset, len, .. } => (offset, len),
                Token::Pause { offset, .. } => (offset, 1),
            })
            .collect();
        assert_eq!(places, [(0, 2), (0, 2), (3, 5), (3, 5), (3, 5), (8, 1)]);
    }

    #[test]
    fn reads_without_a_dictionary() {
        let reading = read("Hello there", None).unwrap();
        assert!(reading.tokens.len() >= 7, "{}", reading.transcript);
    }

    #[test]
    fn stops_at_the_length_limit() {
        let long = "word ".repeat(MAX_TOKENS);
        let error = read(&long, Some(dictionary())).unwrap_err();
        assert_eq!(error.offset % 5, 0);
        assert!(error.offset > 0 && error.offset + error.len == long.len());
    }

    #[test]
    fn empty_text_reads_as_nothing() {
        assert_eq!(read("  ", Some(dictionary())).unwrap(), Reading::default());
    }
}
