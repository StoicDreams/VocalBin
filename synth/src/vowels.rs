//! Vowel targets: the first three formants of American English vowels, from the averages for adult men in
//! Hillenbrand, Getty, Clark and Wheeler (1995), "Acoustic characteristics of American English vowels".
//! Women's and children's voices come from scaling these by vocal tract length, not from separate tables.

pub struct Vowel {
    /// The IPA symbol.
    pub symbol: &'static str,
    /// A word with this vowel, in the study's h_d frame.
    pub example: &'static str,
    pub formants: [f32; 3],
}

/// The order here is the vowel number used by `Synth::set(VOWEL, n)` and the page's vowel buttons.
pub const VOWELS: [Vowel; 12] = [
    Vowel { symbol: "i", example: "heed", formants: [342.0, 2322.0, 3000.0] },
    Vowel { symbol: "ɪ", example: "hid", formants: [427.0, 2034.0, 2684.0] },
    Vowel { symbol: "e", example: "hayed", formants: [476.0, 2089.0, 2691.0] },
    Vowel { symbol: "ɛ", example: "head", formants: [580.0, 1799.0, 2605.0] },
    Vowel { symbol: "æ", example: "had", formants: [588.0, 1952.0, 2601.0] },
    Vowel { symbol: "ʌ", example: "hud", formants: [623.0, 1200.0, 2550.0] },
    Vowel { symbol: "ɑ", example: "hod", formants: [768.0, 1333.0, 2522.0] },
    Vowel { symbol: "ɔ", example: "hawed", formants: [652.0, 997.0, 2538.0] },
    Vowel { symbol: "o", example: "hoed", formants: [497.0, 910.0, 2459.0] },
    Vowel { symbol: "ʊ", example: "hood", formants: [469.0, 1122.0, 2434.0] },
    Vowel { symbol: "u", example: "who'd", formants: [378.0, 997.0, 2343.0] },
    Vowel { symbol: "ɝ", example: "heard", formants: [474.0, 1379.0, 1710.0] },
];

/// The fourth and fifth formants change little between vowels, so they are fixed (for a 17.5 cm tract).
pub const UPPER_FORMANTS: [f32; 2] = [3500.0, 4500.0];

/// Bandwidths of the five formants in Hz, before breathiness widens the first.
pub const BANDWIDTHS: [f32; 5] = [70.0, 100.0, 140.0, 200.0, 250.0];

/// The vocal tract length the tables above describe, in cm.
pub const REFERENCE_TRACT_CM: f32 = 17.5;
