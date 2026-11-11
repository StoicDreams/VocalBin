//! A language, as the engine sees it: its table of sounds, named in IPA, and the other ways people write them.
//! Adding a language means adding a table like `english.rs`. The speaker and the IPA reader stay the same.

use crate::dictionary::Dictionary;
use crate::phonemes::{MAX_TOKENS, ParseError, Phoneme, Token};

/// Text read aloud: the sounds to speak, and the same sounds written in IPA, one word at a time, for people to see.
#[derive(Debug, Default, PartialEq)]
pub struct Reading {
    pub tokens: Vec<Token>,
    pub transcript: String,
}

/// Reads ordinary text in a language, with its pronouncing dictionary when one is loaded.
pub type TextReader = fn(&str, Option<&Dictionary>) -> Result<Reading, ParseError>;

pub struct Language {
    /// A BCP 47 tag, such as `en-US`.
    pub code: &'static str,
    pub name: &'static str,
    /// Every sound the language uses, each named by its IPA symbol.
    pub phonemes: &'static [Phoneme],
    /// Other ways people write these sounds in IPA, each read as one or more of `phonemes` by their symbols. For
    /// example English reads `r` as `ɹ`, and the British `əʊ` as `oʊ`.
    pub aliases: &'static [(&'static str, &'static [&'static str])],
    /// Reads ordinary text: numbers, words, and punctuation, with pronunciations, stress, and intonation. `None`
    /// for a language that can only be written in IPA so far.
    pub read_text: Option<TextReader>,
}

/// How the text given to the speaker is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notation {
    /// IPA, in the speaker's language.
    Ipa,
    /// ARPAbet, the CMU Pronouncing Dictionary's notation. English only.
    Arpabet,
    /// Ordinary text in the speaker's language.
    Text,
}

impl Notation {
    pub fn from_number(n: u32) -> Option<Notation> {
        match n {
            0 => Some(Notation::Arpabet),
            1 => Some(Notation::Ipa),
            2 => Some(Notation::Text),
            _ => None,
        }
    }
}

/// Marks that split text into phrases. `|` and `‖` are IPA's minor and major breaks.
fn pause_mark(c: char) -> Option<char> {
    match c {
        ',' | '|' => Some(','),
        '!' | '?' => Some(c),
        '‖' => Some('.'),
        _ => None,
    }
}

/// Characters that are neither sounds nor marks: brackets around a transcription, and spaces between words.
fn is_gap(c: char) -> bool {
    c.is_whitespace() || matches!(c, '/' | '[' | ']')
}

impl Language {
    /// The sound with this IPA symbol.
    pub fn find(&self, ipa: &str) -> Option<&'static Phoneme> {
        let phonemes: &'static [Phoneme] = self.phonemes;
        phonemes.iter().find(|p| p.ipa == ipa)
    }

    /// The longest sound or alias that `text` starts with, as the sounds it stands for and its length in bytes.
    fn longest_match(&self, text: &str) -> Option<(Vec<&'static Phoneme>, usize)> {
        let phonemes: &'static [Phoneme] = self.phonemes;
        let mut best: Option<(Vec<&'static Phoneme>, usize)> = None;
        let longer = |len: usize, best: &Option<(Vec<&'static Phoneme>, usize)>| best.as_ref().is_none_or(|b| len > b.1);
        for p in phonemes {
            if text.starts_with(p.ipa) && longer(p.ipa.len(), &best) {
                best = Some((vec![p], p.ipa.len()));
            }
        }
        for (alias, sounds) in self.aliases {
            if text.starts_with(alias) && longer(alias.len(), &best) {
                let sounds = sounds.iter().map(|s| self.find(s).expect("an alias names a sound in the table")).collect();
                best = Some((sounds, alias.len()));
            }
        }
        best
    }

    /// Reads IPA in this language, such as `həˈloʊ, ˈwɝld.`.
    ///
    /// - Spaces separate words. Within a word, sounds need no spaces. Slashes and brackets are ignored.
    /// - `ˈ` (or `'`) marks primary stress and `ˌ` secondary stress, on the next vowel. In a word with no stress
    ///   marks, the first vowel that isn't reduced (such as /ə/) is stressed.
    /// - The length marks `ː` and `:` are allowed after a sound and ignored, since length comes from the sound.
    /// - `,` and `|` are short pauses, and `!`, `?`, and `‖` end a sentence. A `.` between two sounds is a
    ///   syllable break and is ignored. Anywhere else it ends a sentence.
    pub fn parse_ipa(&self, text: &str) -> Result<Vec<Token>, ParseError> {
        let mut tokens = Vec::new();
        let mut word = Word::default();
        let mut i = 0;
        while let Some(c) = text[i..].chars().next() {
            let width = c.len_utf8();
            if tokens.len() >= MAX_TOKENS && !is_gap(c) {
                return Err(ParseError { offset: i, len: text.len() - i });
            }
            let is_sound = |c: Option<char>| c.is_some_and(|c| !is_gap(c) && pause_mark(c).is_none() && c != '.');
            if is_gap(c) {
                word.finish(&mut tokens);
            } else if let Some(mark) = pause_mark(c) {
                word.finish(&mut tokens);
                tokens.push(Token::Pause { mark, offset: i });
            } else if c == '.' {
                let syllable_break = is_sound(text[..i].chars().next_back()) && is_sound(text[i + 1..].chars().next());
                if !syllable_break {
                    word.finish(&mut tokens);
                    tokens.push(Token::Pause { mark: '.', offset: i });
                }
            } else if matches!(c, 'ˈ' | '\'') {
                word.stress(1);
            } else if c == 'ˌ' {
                word.stress(2);
            } else if matches!(c, 'ː' | 'ˑ' | ':') && word.has_sound(&tokens) {
                // Length comes from the sound itself.
            } else {
                let (sounds, len) = self.longest_match(&text[i..]).ok_or(ParseError { offset: i, len: width })?;
                for phoneme in sounds {
                    let stress = if phoneme.is_vowel() { word.pending.take().unwrap_or(0) } else { 0 };
                    tokens.push(Token::Phone { phoneme, stress, tone: None, offset: i, len });
                }
                i += len;
                continue;
            }
            i += width;
        }
        word.finish(&mut tokens);
        Ok(tokens)
    }
}

/// The word being read by `parse_ipa`.
#[derive(Default)]
struct Word {
    /// The index in the tokens where the word starts.
    start: usize,
    /// Whether the word has a stress mark.
    marked: bool,
    /// The stress for the word's next vowel.
    pending: Option<u8>,
}

impl Word {
    fn stress(&mut self, level: u8) {
        self.marked = true;
        self.pending = Some(level);
    }

    fn has_sound(&self, tokens: &[Token]) -> bool {
        tokens.len() > self.start
    }

    /// Stresses an unmarked word's first full vowel, and starts the next word.
    fn finish(&mut self, tokens: &mut [Token]) {
        if !self.marked {
            for token in &mut tokens[self.start..] {
                if let Token::Phone { phoneme, stress, .. } = token
                    && phoneme.is_vowel()
                    && !phoneme.reduced
                {
                    *stress = 1;
                    break;
                }
            }
        }
        *self = Word { start: tokens.len(), ..Word::default() };
    }
}
