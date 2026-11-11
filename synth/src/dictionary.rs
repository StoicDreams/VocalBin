//! A pronouncing dictionary: words and how they are said, as sounds in one language's table. Nothing here is
//! specific to a language. The file names its own sounds in IPA, and the language decides what to do with them.
//!
//! The file is UTF-8 text, one line per entry:
//!
//! - Lines starting with `;` are comments, such as the source's license.
//! - `!codes A=ɑ E=æ ...` gives each sound a one-character code, by its IPA symbol in the language's table.
//! - Every other line is a word and its pronunciations, sorted by word in byte order. The word starts with one digit,
//!   the number of characters it shares with the word before it (front coding, to keep the file small). Then come
//!   the rest of the word, a space, and the pronunciations separated by `|`. Each is a run of codes, and a vowel's
//!   code is followed by its stress: `0` none, `1` primary, `2` secondary.
//!
//! For example `0hello hX0lB1` is "hello" as /hə ˈloʊ/, and the next line `5s hX0lB1z` is "hellos".
//!
//! `PowerShell/BuildDictionary.ps1` writes English's file from the CMU Pronouncing Dictionary.

use crate::language::Language;
use crate::phonemes::Phoneme;

/// One sound of a pronunciation, with its stress for vowels.
pub type Sound = (&'static Phoneme, u8);

pub struct Dictionary {
    /// Every word, one after another.
    words: Vec<u8>,
    /// Every pronunciation's codes, one after another.
    codes: Vec<u8>,
    /// For each entry, where its word ends in `words`, and where its pronunciations end in `codes`. Each starts where
    /// the entry before ends.
    entries: Vec<(u32, u32)>,
    /// The sound for each code byte.
    sounds: [Option<&'static Phoneme>; 128],
}

impl Dictionary {
    /// Reads a dictionary file whose sounds are in `language`. Returns what is wrong with it on an error.
    pub fn parse(bytes: &[u8], language: &'static Language) -> Result<Dictionary, String> {
        let text = std::str::from_utf8(bytes).map_err(|_| "the dictionary is not UTF-8".to_string())?;
        let mut dictionary = Dictionary { words: Vec::new(), codes: Vec::new(), entries: Vec::new(), sounds: [None; 128] };
        let mut previous: (usize, usize) = (0, 0);
        for (n, line) in text.lines().enumerate() {
            let error = |what: &str| format!("line {}: {what}", n + 1);
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            if let Some(codes) = line.strip_prefix("!codes ") {
                for pair in codes.split(' ') {
                    let (code, ipa) = pair.split_once('=').ok_or_else(|| error("a code needs code=IPA"))?;
                    let &[code] = code.as_bytes() else { return Err(error("a code is one ASCII character")) };
                    let sound = language.find(ipa).ok_or_else(|| error(&format!("{ipa} is not a sound in {}", language.name)))?;
                    *dictionary.sounds.get_mut(code as usize).ok_or_else(|| error("a code is one ASCII character"))? = Some(sound);
                }
                continue;
            }
            let (word, pronunciations) = line.split_once(' ').ok_or_else(|| error("a word needs pronunciations"))?;
            let shared = word.as_bytes().first().filter(|b| b.is_ascii_digit()).ok_or_else(|| error("a word starts with a digit"))?;
            let shared = (shared - b'0') as usize;
            let (start, end) = previous;
            if shared > end - start {
                return Err(error("shares more than the word before has"));
            }
            let word_start = dictionary.words.len();
            dictionary.words.extend_from_within(start..start + shared);
            dictionary.words.extend_from_slice(&word.as_bytes()[1..]);
            if !dictionary.entries.is_empty() && dictionary.words[start..end] >= dictionary.words[word_start..] {
                return Err(error("words are not in order"));
            }
            for code in pronunciations.bytes().filter(|&b| b != b'|') {
                if !code.is_ascii_digit() && dictionary.sound(code).is_none() {
                    return Err(error(&format!("unknown code {}", code as char)));
                }
            }
            dictionary.codes.extend_from_slice(pronunciations.as_bytes());
            dictionary.entries.push((dictionary.words.len() as u32, dictionary.codes.len() as u32));
            previous = (word_start, dictionary.words.len());
        }
        Ok(dictionary)
    }

    fn sound(&self, code: u8) -> Option<&'static Phoneme> {
        self.sounds.get(code as usize).copied().flatten()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn word(&self, index: usize) -> &[u8] {
        let start = if index == 0 { 0 } else { self.entries[index - 1].0 as usize };
        &self.words[start..self.entries[index].0 as usize]
    }

    /// Whether the dictionary has `word`, written as it is stored (English stores lower case).
    pub fn contains(&self, word: &str) -> bool {
        self.find(word).is_some()
    }

    fn find(&self, word: &str) -> Option<usize> {
        let (mut low, mut high) = (0, self.entries.len());
        while low < high {
            let middle = (low + high) / 2;
            match self.word(middle).cmp(word.as_bytes()) {
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
                std::cmp::Ordering::Equal => return Some(middle),
            }
        }
        None
    }

    /// Every pronunciation of `word`, most common first. Empty when the dictionary doesn't have it.
    pub fn lookup(&self, word: &str) -> Vec<Vec<Sound>> {
        let Some(index) = self.find(word) else { return Vec::new() };
        let start = if index == 0 { 0 } else { self.entries[index - 1].1 as usize };
        let codes = &self.codes[start..self.entries[index].1 as usize];
        codes
            .split(|&b| b == b'|')
            .map(|pronunciation| {
                let mut sounds: Vec<Sound> = Vec::with_capacity(pronunciation.len());
                for &code in pronunciation {
                    if code.is_ascii_digit() {
                        if let Some(last) = sounds.last_mut() {
                            last.1 = code - b'0';
                        }
                    } else if let Some(sound) = self.sound(code) {
                        sounds.push((sound, 0));
                    }
                }
                sounds
            })
            .collect()
    }
}

/// The English dictionary the site serves, read once for tests.
#[cfg(test)]
pub fn english_for_tests() -> &'static Dictionary {
    static DICTIONARY: std::sync::OnceLock<Dictionary> = std::sync::OnceLock::new();
    DICTIONARY.get_or_init(|| {
        Dictionary::parse(include_bytes!("../../webapp/root_files/vocal-synth-en-us.dict"), &crate::english::ENGLISH).unwrap()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::english::ENGLISH;

    fn ipa(sounds: &[Sound]) -> String {
        sounds.iter().map(|(p, s)| if p.is_vowel() { format!("{}{s}", p.ipa) } else { p.ipa.to_string() }).collect::<Vec<_>>().join(" ")
    }

    const SMALL: &str = "; a comment\n!codes X=ə B=oʊ h=h l=l z=z H=ɛ d=d L=i r=ɹ\n0hello hX0lB1\n5s hX0lB1z\n0read rH1d|rL1d\n";

    #[test]
    fn reads_front_coded_words() {
        let d = Dictionary::parse(SMALL.as_bytes(), &ENGLISH).unwrap();
        assert_eq!(d.len(), 3);
        assert_eq!(ipa(&d.lookup("hellos")[0]), "h ə0 l oʊ1 z");
        assert_eq!(d.lookup("read").iter().map(|p| ipa(p)).collect::<Vec<_>>(), ["ɹ ɛ1 d", "ɹ i1 d"]);
        assert!(d.lookup("help").is_empty() && d.lookup("").is_empty() && d.lookup("zzz").is_empty());
    }

    #[test]
    fn rejects_broken_files() {
        let parse = |text: &str| Dictionary::parse(text.as_bytes(), &ENGLISH).err();
        assert!(parse("!codes X=ə\n0b X0\n0a X0\n").unwrap().contains("order"));
        assert!(parse("!codes X=ə\n0a Q0\n").unwrap().contains("unknown code"));
        assert!(parse("!codes X=ʘ\n").unwrap().contains("not a sound"));
        assert!(parse("!codes X=ə\n0a X0\n3b X0\n").unwrap().contains("shares more"));
    }

    #[test]
    fn reads_the_english_dictionary() {
        let d = english_for_tests();
        assert!(d.len() > 120_000, "{} words", d.len());
        assert_eq!(ipa(&d.lookup("hello")[0]), "h ə0 l oʊ1");
        assert_eq!(ipa(&d.lookup("speech")[0]), "s p i1 tʃ");
        assert_eq!(ipa(&d.lookup("don't")[0]), "d oʊ1 n t");
        assert!(d.contains("zebra") && d.contains("'bout") && !d.contains("vlog"));
    }
}
