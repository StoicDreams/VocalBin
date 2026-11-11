//! What a speech sound is to the engine, independent of any language. Each sound is named by its IPA symbol and
//! described by how it is made (its [`Kind`] and [`Place`]) and its acoustic targets. A language brings its own
//! table of these (see `language.rs` and `english.rs`), so another language's version of a sound can have its own
//! values, and the speaker never needs to know which language it is speaking.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Vowel,
    /// A vowel that moves from one target to another, as in English /aɪ/ (hide).
    Diphthong,
    /// Voiced, vowel-like consonants with slower transitions, such as /l/, /ɹ/, /w/, and /j/.
    Approximant,
    /// Voiced through the nose, with an extra pole and zero, such as /m/, /n/, and /ŋ/.
    Nasal,
    /// Hiss from a narrow gap, such as /s/, /ʃ/, /f/, and /θ/, and their voiced pairs.
    Fricative,
    /// A closure, a burst, then aspiration or voicing, such as /p/, /t/, /k/, /b/, /d/, and /ɡ/.
    Plosive,
    /// A plosive released into a fricative, such as /tʃ/ and /dʒ/.
    Affricate,
    /// /h/: breath through the shape of the vowel that follows.
    Aspirate,
}

/// Where a consonant is made. It sets the burst's spectrum, and velars move with the vowel that follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// Vowels.
    None,
    /// The lips, or the lips and teeth.
    Labial,
    /// The tongue tip at the teeth.
    Dental,
    /// The tongue tip at the ridge behind the teeth.
    Alveolar,
    /// The tongue blade just behind the ridge.
    Postalveolar,
    /// The tongue body at the hard palate.
    Palatal,
    /// The tongue back at the soft palate.
    Velar,
    /// The vocal folds.
    Glottal,
}

#[derive(Debug, PartialEq)]
pub struct Phoneme {
    /// The IPA symbol, such as `ʃ` or `aɪ`.
    pub ipa: &'static str,
    pub kind: Kind,
    pub place: Place,
    pub voiced: bool,
    /// A vowel that is only ever unstressed, such as /ə/. IPA text without stress marks never stresses it.
    pub reduced: bool,
    /// F1 to F3 in Hz: the vowel target, or the consonant's locus.
    pub formants: [f32; 3],
    /// Where a diphthong ends. The same as `formants` for every other phoneme.
    pub end: [f32; 3],
    /// Length in ms at normal speed. For plosives and affricates, the closure.
    pub ms: f32,
    /// Plosives: the voice onset time after the burst. Affricates: the length of the fricative part.
    pub release_ms: f32,
    /// Fricatives and affricates: the hiss, as levels in the parallel bands F2, F3, F4, F5, F6, and bypass.
    pub noise: [f32; 6],
    /// Nasals: the nasal zero in Hz.
    pub nasal_zero: f32,
}

impl Phoneme {
    pub fn is_vowel(&self) -> bool {
        matches!(self.kind, Kind::Vowel | Kind::Diphthong)
    }
}

/// A neutral vocal tract: where formants rest in silence and head after a consonant with no vowel to go to.
pub const NEUTRAL: [f32; 3] = [500.0, 1500.0, 2500.0];

const QUIET: [f32; 6] = [0.0; 6];
/// The hiss of /s/ and /z/: high and strong, from F5 and F6.
pub const S_NOISE: [f32; 6] = [0.0, 0.0, 0.1, 0.4, 1.0, 0.1];
/// The hiss of /ʃ/ and /ʒ/: lower, peaking at F3, and the loudest fricative.
pub const SH_NOISE: [f32; 6] = [0.6, 2.0, 1.2, 0.6, 0.2, 0.0];
/// The weak, flat hiss of /f/ and /v/.
pub const F_NOISE: [f32; 6] = [0.0, 0.0, 0.0, 0.025, 0.04, 0.09];
/// The weak hiss of /θ/ and /ð/.
pub const TH_NOISE: [f32; 6] = [0.0, 0.0, 0.0, 0.04, 0.05, 0.075];

/// The burst that releases a closure at `place`: flat and weak at the lips, high at the tongue tip, and compact
/// around F2 and F3 at the tongue back.
pub fn burst(place: Place) -> [f32; 6] {
    match place {
        Place::Labial => [0.0, 0.0, 0.0, 0.0, 0.2, 0.6],
        Place::Palatal | Place::Velar => [1.0, 0.7, 0.3, 0.0, 0.0, 0.0],
        _ => [0.0, 0.0, 0.3, 0.6, 1.0, 0.3],
    }
}

// Builders for language tables.

pub const fn base(ipa: &'static str, kind: Kind, place: Place, voiced: bool, formants: [f32; 3], ms: f32) -> Phoneme {
    Phoneme { ipa, kind, place, voiced, reduced: false, formants, end: formants, ms, release_ms: 0.0, noise: QUIET, nasal_zero: 0.0 }
}

pub const fn vowel(ipa: &'static str, formants: [f32; 3], ms: f32) -> Phoneme {
    base(ipa, Kind::Vowel, Place::None, true, formants, ms)
}

/// A vowel that is only ever unstressed, such as /ə/.
pub const fn reduced(ipa: &'static str, formants: [f32; 3], ms: f32) -> Phoneme {
    Phoneme { reduced: true, ..vowel(ipa, formants, ms) }
}

pub const fn diphthong(ipa: &'static str, formants: [f32; 3], end: [f32; 3], ms: f32) -> Phoneme {
    Phoneme { end, ..base(ipa, Kind::Diphthong, Place::None, true, formants, ms) }
}

pub const fn approximant(ipa: &'static str, place: Place, formants: [f32; 3], ms: f32) -> Phoneme {
    base(ipa, Kind::Approximant, place, true, formants, ms)
}

pub const fn nasal(ipa: &'static str, place: Place, formants: [f32; 3], zero: f32, ms: f32) -> Phoneme {
    Phoneme { nasal_zero: zero, ..base(ipa, Kind::Nasal, place, true, formants, ms) }
}

pub const fn fricative(ipa: &'static str, place: Place, voiced: bool, formants: [f32; 3], noise: [f32; 6], ms: f32) -> Phoneme {
    Phoneme { noise, ..base(ipa, Kind::Fricative, place, voiced, formants, ms) }
}

pub const fn plosive(ipa: &'static str, place: Place, voiced: bool, formants: [f32; 3], closure_ms: f32, vot_ms: f32) -> Phoneme {
    Phoneme { release_ms: vot_ms, ..base(ipa, Kind::Plosive, place, voiced, formants, closure_ms) }
}

pub const fn affricate(ipa: &'static str, place: Place, voiced: bool, formants: [f32; 3], noise: [f32; 6], closure_ms: f32, hiss_ms: f32) -> Phoneme {
    Phoneme { noise, release_ms: hiss_ms, ..base(ipa, Kind::Affricate, place, voiced, formants, closure_ms) }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Token {
    /// A phoneme, with its stress for vowels (0 none, 1 primary, 2 secondary), and where it is in the text.
    Phone { phoneme: &'static Phoneme, stress: u8, offset: usize, len: usize },
    /// `,` is a short pause. `.`, `!`, and `?` end a sentence, and `?` ends it with a rise.
    Pause { mark: char, offset: usize },
}

/// Where in the text, in bytes, the first thing that can't be read is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub offset: usize,
    pub len: usize,
}

/// The most phonemes and pauses one text may have.
pub const MAX_TOKENS: usize = 2000;
