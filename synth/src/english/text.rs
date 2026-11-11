//! Turns English text into the words to say and where to pause: numbers, money, dates, times, abbreviations,
//! symbols, acronyms, and web addresses become words, and punctuation becomes pauses.
//!
//! Each word keeps the place in the text it came from, so the page can highlight it while it is spoken. Words made
//! from one thing in the text, such as the four words of "$5.50", all point at it.

use super::numbers;

#[derive(Clone, Debug, PartialEq)]
pub struct Word {
    /// Lower case, with straight apostrophes and no accents.
    pub text: String,
    pub offset: usize,
    pub len: usize,
    /// Say each letter's name, as for "FBI".
    pub spell: bool,
    /// Written in capitals in the text, such as "NASA".
    pub caps: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Piece {
    Word(Word),
    /// `,` for a short pause, or `.`, `!`, or `?` for the end of a sentence.
    Break { mark: char, offset: usize },
}

/// Abbreviations read as words when written with a period, and whether one can end a sentence. Titles such as
/// "Dr." can't, because a name follows.
const ABBREVIATIONS: [(&str, &str, bool); 47] = [
    ("mr", "mister", false),
    ("mrs", "missus", false),
    ("ms", "miz", false),
    ("dr", "doctor", false),
    ("prof", "professor", false),
    ("gov", "governor", false),
    ("sen", "senator", false),
    ("rep", "representative", false),
    ("gen", "general", false),
    ("capt", "captain", false),
    ("lt", "lieutenant", false),
    ("sgt", "sergeant", false),
    ("col", "colonel", false),
    ("mt", "mount", false),
    ("vs", "versus", false),
    ("approx", "approximately", false),
    ("fig", "figure", false),
    ("vol", "volume", false),
    ("no", "number", false),
    ("e.g", "for example", false),
    ("i.e", "that is", false),
    ("jr", "junior", true),
    ("sr", "senior", true),
    ("st", "street", true),
    ("ave", "avenue", true),
    ("blvd", "boulevard", true),
    ("rd", "road", true),
    ("hwy", "highway", true),
    ("etc", "et cetera", true),
    ("inc", "incorporated", true),
    ("ltd", "limited", true),
    ("co", "company", true),
    ("corp", "corporation", true),
    ("dept", "department", true),
    ("jan", "january", true),
    ("feb", "february", true),
    ("mar", "march", true),
    ("apr", "april", true),
    ("jun", "june", true),
    ("jul", "july", true),
    ("aug", "august", true),
    ("sep", "september", true),
    ("sept", "september", true),
    ("oct", "october", true),
    ("nov", "november", true),
    ("dec", "december", true),
    ("min", "minutes", true),
];

/// Units read after a number, as singular and plural.
const UNITS: [(&str, &str, &str); 30] = [
    ("km", "kilometer", "kilometers"),
    ("cm", "centimeter", "centimeters"),
    ("mm", "millimeter", "millimeters"),
    ("kg", "kilogram", "kilograms"),
    ("mg", "milligram", "milligrams"),
    ("g", "gram", "grams"),
    ("lb", "pound", "pounds"),
    ("lbs", "pound", "pounds"),
    ("oz", "ounce", "ounces"),
    ("ft", "foot", "feet"),
    ("mi", "mile", "miles"),
    ("mph", "mile per hour", "miles per hour"),
    ("kph", "kilometer per hour", "kilometers per hour"),
    ("ml", "milliliter", "milliliters"),
    ("ms", "millisecond", "milliseconds"),
    ("hz", "hertz", "hertz"),
    ("khz", "kilohertz", "kilohertz"),
    ("mhz", "megahertz", "megahertz"),
    ("ghz", "gigahertz", "gigahertz"),
    ("kb", "kilobyte", "kilobytes"),
    ("mb", "megabyte", "megabytes"),
    ("gb", "gigabyte", "gigabytes"),
    ("tb", "terabyte", "terabytes"),
    ("x", "times", "times"),
    ("hr", "hour", "hours"),
    ("hrs", "hour", "hours"),
    ("min", "minute", "minutes"),
    ("mins", "minute", "minutes"),
    ("sec", "second", "seconds"),
    ("secs", "second", "seconds"),
];

/// Units that may also follow a number as their own word, as in "5 km". The others, such as "g" and "x", are only
/// units when written against the number.
fn is_separate_unit(unit: &str) -> bool {
    !matches!(unit, "g" | "mi" | "ms" | "x")
}

const CURRENCIES: [(char, &str, &str); 4] = [('$', "dollar", "dollars"), ('€', "euro", "euros"), ('£', "pound", "pounds"), ('¥', "yen", "yen")];

const SCALE_WORDS: [&str; 4] = ["thousand", "million", "billion", "trillion"];

/// Words that often start a sentence. After "U.S." or other initials, a capital starting one of these is taken as a
/// new sentence. Any other capital is a name: "the U.S. Army".
const SENTENCE_STARTS: [&str; 44] = [
    "the", "a", "an", "i", "we", "he", "she", "it", "they", "you", "this", "that", "these", "those", "there", "in", "on",
    "at", "but", "and", "so", "if", "when", "what", "why", "how", "our", "my", "his", "her", "their", "its", "as", "for",
    "after", "before", "then", "some", "many", "most", "all", "no", "yes", "today",
];

/// Words before a Roman numeral that is read as a plain number: "Chapter IV", "World War II".
const NUMBERED: [&str; 18] = [
    "chapter", "part", "volume", "book", "act", "scene", "section", "phase", "stage", "level", "war", "type", "class",
    "article", "appendix", "episode", "season", "bowl",
];

/// A Roman numeral of I, V, and X letters, up to 39, such as "IV" and "XIII". `None` for anything else.
fn roman(letters: &str) -> Option<u64> {
    let value = |c| match c {
        'I' => Some(1),
        'V' => Some(5),
        'X' => Some(10),
        _ => None,
    };
    let digits: Vec<u64> = letters.chars().map(value).collect::<Option<_>>()?;
    let mut total = 0;
    for (i, &d) in digits.iter().enumerate() {
        if digits.get(i + 1).is_some_and(|&next| next > d) {
            total -= d as i64;
        } else {
            total += d as i64;
        }
    }
    // Only the standard way of writing it: "IIII" and "VX" are not numerals.
    let total = u64::try_from(total).ok().filter(|&t| (1..40).contains(&t))?;
    let tens = "X".repeat((total / 10) as usize);
    let ones = ["", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX"][(total % 10) as usize];
    (format!("{tens}{ones}") == letters).then_some(total)
}

/// Symbols said as words.
fn symbol_word(c: char) -> Option<&'static str> {
    Some(match c {
        '&' => "and",
        '+' => "plus",
        '=' => "equals",
        '@' => "at",
        '%' => "percent",
        '°' => "degrees",
        '×' => "times",
        '~' => "about",
        '<' => "less than",
        '>' => "greater than",
        '#' => "number",
        _ => return None,
    })
}

/// Marks that open a parenthetical, heard as a short pause before it.
fn opens(c: char) -> bool {
    matches!(c, '(' | '[' | '{')
}

/// Characters stripped from the start of a chunk of text.
fn is_leading(c: char) -> bool {
    opens(c) || matches!(c, '"' | '\'' | '“' | '‘' | '«' | '¿' | '¡' | '*' | '_')
}

/// Characters stripped from the end of a chunk of text.
fn is_trailing(c: char) -> bool {
    matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | '…' | ')' | ']' | '}' | '"' | '\'' | '”' | '’' | '»' | '*' | '_' | '—' | '–')
}

/// The pause that trailing punctuation asks for, strongest first.
fn mark_of(trailing: &str) -> Option<char> {
    if trailing.contains('?') {
        Some('?')
    } else if trailing.contains('!') {
        Some('!')
    } else if trailing.contains(['.', '…']) {
        Some('.')
    } else if trailing.contains([',', ';', ':', ')', ']', '}', '—', '–']) {
        Some(',')
    } else {
        None
    }
}

/// Lower case without accents, for looking a word up.
fn plain(word: &str) -> String {
    word.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            '’' | '‘' => '\'',
            c => c,
        })
        .collect()
}

/// A run of characters that are not spaces, and where it starts.
struct Chunk<'a> {
    text: &'a str,
    offset: usize,
}

fn chunks(text: &str) -> Vec<Chunk<'_>> {
    let mut chunks = Vec::new();
    let mut start = None;
    for (i, c) in text.char_indices() {
        // Dashes between words separate them, like spaces.
        let gap = c.is_whitespace() || c == '—' || c == '–';
        match (gap, start) {
            (true, Some(s)) => {
                chunks.push(Chunk { text: &text[s..i], offset: s });
                if c != ' ' && !c.is_whitespace() {
                    chunks.push(Chunk { text: &text[i..i + c.len_utf8()], offset: i });
                }
                start = None;
            }
            (true, None) if !c.is_whitespace() => chunks.push(Chunk { text: &text[i..i + c.len_utf8()], offset: i }),
            (false, None) => start = Some(i),
            _ => {}
        }
    }
    if let Some(s) = start {
        chunks.push(Chunk { text: &text[s..], offset: s });
    }
    chunks
}

/// One part of a chunk: a word, a number, or a symbol, and where it is.
#[derive(Clone, Debug, PartialEq)]
enum Part<'a> {
    Letters(&'a str, usize),
    Number(&'a str, usize),
    Symbol(char, usize),
}

impl Part<'_> {
    fn span(&self) -> (usize, usize) {
        match *self {
            Part::Letters(s, o) | Part::Number(s, o) => (o, s.len()),
            Part::Symbol(c, o) => (o, c.len_utf8()),
        }
    }
}

/// Splits a chunk into words, numbers, and symbols. A word keeps apostrophes and dots between letters ("don't",
/// "e.g"), and a number keeps commas, dots, and colons between digits ("1,000", "3.14", "3:30").
fn parts(text: &str, offset: usize) -> Vec<Part<'_>> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let at = |i: usize| chars.get(i).map(|&(_, c)| c);
    let mut parts = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let (start, c) = chars[i];
        let mut j = i + 1;
        if c.is_alphabetic() {
            while let Some(c) = at(j) {
                let joins = matches!(c, '\'' | '’' | '.') && at(j + 1).is_some_and(char::is_alphabetic);
                if c.is_alphabetic() || joins {
                    j += if joins { 2 } else { 1 };
                } else {
                    break;
                }
            }
            let end = chars.get(j).map_or(text.len(), |&(o, _)| o);
            parts.push(Part::Letters(&text[start..end], offset + start));
        } else if c.is_ascii_digit() || (c == '.' && at(j).is_some_and(|c| c.is_ascii_digit())) {
            while let Some(c) = at(j) {
                let joins = matches!(c, ',' | '.' | ':') && at(j + 1).is_some_and(|c| c.is_ascii_digit());
                if c.is_ascii_digit() || joins {
                    j += if joins { 2 } else { 1 };
                } else {
                    break;
                }
            }
            let end = chars.get(j).map_or(text.len(), |&(o, _)| o);
            parts.push(Part::Number(&text[start..end], offset + start));
        } else {
            parts.push(Part::Symbol(c, offset + start));
        }
        i = j;
    }
    parts
}

/// Builds the pieces for a text.
struct Reader<'a> {
    chunks: Vec<Chunk<'a>>,
    pieces: Vec<Piece>,
    /// The number just said, so a following unit ("5 km") or scale word can agree with it.
    last_number: Option<f64>,
    /// A currency waiting for a scale word, as in "$5 million" "five million dollars".
    currency: Option<&'static str>,
}

impl<'a> Reader<'a> {
    fn say(&mut self, words: &[&str], (offset, len): (usize, usize)) {
        for word in words.iter().flat_map(|w| w.split(' ')) {
            self.pieces.push(Piece::Word(Word { text: word.to_string(), offset, len, spell: false, caps: false }));
        }
        self.last_number = None;
    }

    fn say_all(&mut self, words: Vec<String>, span: (usize, usize)) {
        let words: Vec<&str> = words.iter().map(String::as_str).collect();
        self.say(&words, span);
    }

    fn say_number(&mut self, words: Vec<String>, value: f64, span: (usize, usize)) {
        self.say_all(words, span);
        self.last_number = Some(value);
    }

    fn spell(&mut self, letters: &str, (offset, len): (usize, usize)) {
        for c in plain(letters).chars().filter(|c| c.is_alphabetic()) {
            self.pieces.push(Piece::Word(Word { text: c.to_string(), offset, len, spell: true, caps: true }));
        }
        self.last_number = None;
    }

    fn pause(&mut self, mark: char, offset: usize) {
        match self.pieces.last_mut() {
            None => {}
            // Keep the stronger of two pauses in a row.
            Some(Piece::Break { mark: last, .. }) => {
                if *last == ',' {
                    *last = mark;
                }
            }
            Some(_) => self.pieces.push(Piece::Break { mark, offset }),
        }
        self.last_number = None;
    }

    /// The next chunk's text, lower case without trailing punctuation, for looking ahead.
    fn peek(&self, index: usize) -> Option<String> {
        self.chunks.get(index + 1).map(|c| plain(c.text.trim_end_matches(is_trailing)))
    }

    fn read(mut self) -> Vec<Piece> {
        for index in 0..self.chunks.len() {
            let chunk = &self.chunks[index];
            let (mut text, mut offset) = (chunk.text, chunk.offset);
            while let Some(c) = text.chars().next().filter(|&c| is_leading(c)) {
                if opens(c) {
                    self.pause(',', offset);
                }
                text = &text[c.len_utf8()..];
                offset += c.len_utf8();
            }
            let core = text.trim_end_matches(is_trailing);
            let trailing = &text[core.len()..];
            let mut mark = mark_of(trailing);
            let mark_offset = offset + core.len() + trailing.find(['.', ',', ';', ':', '!', '?', '…', ')', ']', '}', '—', '–']).unwrap_or(0);
            let next_capital = self.chunks.get(index + 1).is_some_and(|c| c.text.trim_start_matches(is_leading).starts_with(char::is_uppercase));
            let previous_capital = index > 0 && self.chunks[index - 1].text.trim_start_matches(is_leading).starts_with(char::is_uppercase);
            let last = index + 1 == self.chunks.len();

            if trailing.starts_with('.') && !core.is_empty() {
                let key = plain(core);
                if let Some(&(_, words, can_end)) = ABBREVIATIONS.iter().find(|a| a.0 == key) {
                    // "No." is only "number" before a number.
                    let before_number = self.chunks.get(index + 1).is_some_and(|c| c.text.starts_with(|c: char| c.is_ascii_digit()));
                    if key != "no" || before_number {
                        // "St." before a name is "Saint", unless it follows one: "Main St.". "Dr." after a name is "Drive",
                        // unless a name follows it: "Oak Dr.", "Dr. Smith".
                        let words = if key == "st" && next_capital && !last && !previous_capital { "saint" } else { words };
                        let drive = key == "dr" && previous_capital && (last || !next_capital);
                        let (words, can_end) = if drive { ("drive", true) } else { (words, can_end) };
                        self.say(&[words], (offset, core.len()));
                        if trailing == "." && !(can_end && (last || next_capital)) || key == "st" && words == "saint" {
                            mark = None;
                        }
                        if let Some(mark) = mark {
                            self.pause(mark, mark_offset);
                        }
                        continue;
                    }
                }
                // A single letter, or letters joined by dots, with a period: initials, "J." or "U.S.".
                let initials = core.split('.').all(|p| p.chars().count() == 1 && p.chars().all(char::is_alphabetic));
                if initials {
                    self.spell(core, (offset, core.len()));
                    // "in the U.S. Army" goes on; "in the U.S. The next day" is a new sentence.
                    let starts_sentence = self.peek(index).is_some_and(|w| SENTENCE_STARTS.contains(&w.as_str()));
                    if trailing == "." && !(last || next_capital && core.contains('.') && starts_sentence) {
                        mark = None;
                    }
                    if core.chars().count() == 1 && !last {
                        mark = mark.filter(|_| trailing != ".");
                    }
                    if let Some(mark) = mark {
                        self.pause(mark, mark_offset);
                    }
                    continue;
                }
            }

            self.read_core(core, offset, index);
            if let Some(mark) = mark {
                self.pause(mark, mark_offset);
            }
        }
        // Drop a pause at the very start, and end with a sentence.
        if let Some(Piece::Break { mark: ',', .. }) = self.pieces.last() {
            self.pieces.pop();
        }
        self.pieces
    }

    fn read_core(&mut self, core: &'a str, offset: usize, index: usize) {
        if core.is_empty() {
            return;
        }
        let lower = core.to_lowercase();
        if lower.contains("://") || lower.starts_with("www.") || (core.contains('@') && core.contains('.')) {
            self.read_address(core, offset);
            return;
        }
        let parts = parts(core, offset);
        let mut i = 0;
        while i < parts.len() {
            i += self.read_part(&parts, i, index);
        }
    }

    /// Reads web and email addresses: "dot", "slash", and "at" between the words.
    fn read_address(&mut self, core: &str, offset: usize) {
        let span = (offset, core.len());
        let address = core.split_once("://").map_or(core, |(_, rest)| rest).trim_end_matches('/');
        let mut word = String::new();
        let flush = |word: &mut String, reader: &mut Reader| {
            if !word.is_empty() {
                if word.chars().all(|c| c.is_ascii_digit()) {
                    reader.say_all(numbers::digits(word), span);
                } else {
                    reader.pieces.push(Piece::Word(Word { text: plain(word), offset, len: core.len(), spell: false, caps: false }));
                }
                word.clear();
            }
        };
        for c in address.chars() {
            let name = match c {
                '.' => Some("dot"),
                '/' => Some("slash"),
                '@' => Some("at"),
                '-' => Some("dash"),
                '_' => Some("underscore"),
                ':' => Some("colon"),
                _ => None,
            };
            if let Some(name) = name {
                flush(&mut word, self);
                self.say(&[name], span);
            } else if c.is_alphanumeric() {
                word.push(c);
            } else {
                flush(&mut word, self);
            }
        }
        flush(&mut word, self);
    }

    /// Reads the part at `i` and any parts it joins with, such as "$" with "5". Returns how many parts it used.
    fn read_part(&mut self, parts: &[Part<'a>], i: usize, index: usize) -> usize {
        let next = parts.get(i + 1);
        let span_to = |last: usize| {
            let (start, _) = parts[i].span();
            let (o, l) = parts[last].span();
            (start, o + l - start)
        };
        match parts[i] {
            Part::Symbol(c, _) if CURRENCIES.iter().any(|x| x.0 == c) => {
                let Some(&Part::Number(amount, _)) = next else {
                    if let Some(name) = CURRENCIES.iter().find(|x| x.0 == c) {
                        self.say(&[name.2], parts[i].span());
                    }
                    return 1;
                };
                let (_, one, many) = *CURRENCIES.iter().find(|x| x.0 == c).expect("a currency");
                let span = span_to(i + 1);
                let scale = self.peek(index).filter(|w| SCALE_WORDS.contains(&w.as_str()));
                if let Some((whole, cents)) = amount.split_once('.').filter(|(_, f)| f.len() == 2 && c == '$' && scale.is_none()) {
                    let dollars = numbers::parse(whole).unwrap_or(0);
                    let cents = cents.parse::<u64>().unwrap_or(0);
                    let mut words = Vec::new();
                    if dollars > 0 || cents == 0 {
                        words.extend(numbers::cardinal(dollars));
                        words.push(if dollars == 1 { one } else { many }.to_string());
                    }
                    if cents > 0 {
                        if dollars > 0 {
                            words.push("and".into());
                        }
                        words.extend(numbers::cardinal(cents));
                        words.push(if cents == 1 { "cent" } else { "cents" }.into());
                    }
                    self.say_all(words, span);
                } else {
                    let value = amount.replace(',', "").parse::<f64>().unwrap_or(0.0);
                    self.say_all(numbers::number(amount), span);
                    if scale.is_some() {
                        self.currency = Some(many);
                    } else {
                        self.say(&[if value == 1.0 { one } else { many }], span);
                    }
                }
                2
            }
            Part::Symbol('-' | '−', _) if i == 0 && matches!(next, Some(Part::Number(..))) => {
                self.say(&["minus"], parts[i].span());
                1
            }
            Part::Symbol('#', _) if !matches!(next, Some(Part::Number(..))) => 1,
            Part::Symbol('°', _) if matches!(next, Some(&Part::Letters(l, _)) if matches!(l, "F" | "C")) => {
                let scale = if matches!(next, Some(Part::Letters("F", _))) { "fahrenheit" } else { "celsius" };
                self.say(&["degrees", scale], span_to(i + 1));
                2
            }
            Part::Symbol('%', _) if i > 0 && matches!(parts[i - 1], Part::Number(..)) => {
                self.say(&["percent"], parts[i].span());
                1
            }
            Part::Symbol(c, _) => {
                if let Some(word) = symbol_word(c) {
                    self.say(&[word], parts[i].span());
                }
                1
            }
            Part::Number(number, _) => self.read_number(parts, i, index, number),
            Part::Letters(letters, _) => {
                let key = plain(letters);
                let span = parts[i].span();
                if let Some(value) = self.last_number.filter(|_| i == 0)
                    && let Some(&(unit, one, many)) = UNITS.iter().find(|u| u.0 == key)
                    && is_separate_unit(unit)
                {
                    self.say(&[if value == 1.0 { one } else { many }], span);
                    return 1;
                }
                if let Some(currency) = self.currency.take()
                    && SCALE_WORDS.contains(&key.as_str())
                {
                    self.say(&[&key, currency], span);
                    return 1;
                }
                if matches!(key.as_str(), "vs" | "etc") {
                    let words = ABBREVIATIONS.iter().find(|a| a.0 == key).map_or("", |a| a.1);
                    self.say(&[words], span);
                    return 1;
                }
                if matches!(key.as_str(), "am" | "pm") && letters.chars().all(|c| c.is_uppercase()) || key == "a.m" || key == "p.m" {
                    self.spell(letters, span);
                    return 1;
                }
                if key.contains('.') {
                    // "e.g", or letters joined by dots: "U.S", "node.js".
                    if let Some(&(_, words, _)) = ABBREVIATIONS.iter().find(|a| a.0 == key) {
                        self.say(&[words], span);
                    } else if key.split('.').all(|p| p.chars().count() == 1) {
                        self.spell(letters, span);
                    } else {
                        for (n, part) in key.split('.').enumerate() {
                            if n > 0 {
                                self.say(&["dot"], span);
                            }
                            self.word(part, span, false);
                        }
                    }
                    return 1;
                }
                // A Roman numeral after a name is an ordinal, "Charles III" "Charles the third", and after a word
                // such as "Chapter" a number, "Chapter IV" "chapter four".
                if i == 0
                    && index > 0
                    && let Some(n) = roman(letters).filter(|_| letters.len() >= 2 || letters == "V")
                {
                    let previous = &self.chunks[index - 1].text;
                    let previous_word = plain(previous.trim_end_matches(is_trailing));
                    if previous.ends_with(char::is_alphabetic) && NUMBERED.contains(&previous_word.as_str()) {
                        self.say_all(numbers::cardinal(n), span);
                        return 1;
                    }
                    if previous.ends_with(char::is_alphabetic) && previous.starts_with(char::is_uppercase) && previous_word.len() > 1 {
                        let mut words = vec!["the".to_string()];
                        words.extend(numbers::ordinal(n));
                        self.say_all(words, span);
                        return 1;
                    }
                }
                let caps = letters.chars().count() > 1 && letters.chars().all(|c| !c.is_lowercase());
                self.word(&key, span, caps);
                // Letters straight before a number, as in "MP3", are spelled when short and in capitals.
                1
            }
        }
    }

    fn word(&mut self, text: &str, (offset, len): (usize, usize), caps: bool) {
        if !text.is_empty() {
            self.pieces.push(Piece::Word(Word { text: text.to_string(), offset, len, spell: false, caps }));
        }
        self.last_number = None;
    }

    fn read_number(&mut self, parts: &[Part<'a>], i: usize, index: usize, number: &str) -> usize {
        let span = parts[i].span();
        let next = parts.get(i + 1);
        let after = parts.get(i + 2);
        let span_to = |last: usize| {
            let (o, l) = parts[last].span();
            (span.0, o + l - span.0)
        };
        let value = number.replace(',', "").parse::<f64>().ok();

        // A time, "3:30", "10:05 pm". Minutes always have two digits, so "3:1" is a ratio, "three to one".
        if let Some((h_text, m_text)) = number.split_once(':')
            && let (Ok(h), Ok(m)) = (h_text.parse::<u64>(), m_text.parse::<u64>())
            && number.split(':').count() == 2
        {
            if m_text.len() == 2 && h < 24 && m < 60 {
                let am_pm = matches!(next, Some(Part::Letters(l, _)) if matches!(plain(l).as_str(), "am" | "pm" | "a.m" | "p.m"))
                    || self.peek(index).is_some_and(|w| matches!(w.as_str(), "am" | "pm" | "a.m" | "p.m"));
                self.say_all(numbers::time(h, m, am_pm), span);
                return 1;
            }
            if h_text.len() <= 2 && m_text.len() <= 2 {
                let mut words = numbers::cardinal(h);
                words.push("to".into());
                words.extend(numbers::cardinal(m));
                self.say_all(words, span);
                return 1;
            }
        }
        if number.contains(':') {
            for (n, piece) in number.split(':').enumerate() {
                if n > 0 {
                    self.pause(',', span.0);
                }
                self.say_all(numbers::number(piece), span);
            }
            return 1;
        }

        // Joined to another number by "/" or "-": dates, fractions, ranges, and phone numbers.
        if let (Some(Part::Symbol(join @ ('/' | '-' | '–'), _)), Some(&Part::Number(second, _))) = (next, after) {
            let third = match (parts.get(i + 3), parts.get(i + 4)) {
                (Some(Part::Symbol(j, _)), Some(&Part::Number(t, _))) if j == join => Some(t),
                _ => None,
            };
            let whole = |s: &str| (!s.contains(['.', ','])).then(|| s.parse::<u64>().ok()).flatten();
            let (a, b) = (whole(number), whole(second));
            if let (Some(t), Some(a), Some(b)) = (third, a, b) {
                let c = whole(t);
                // 11/11/2026 (month first), or 2026-11-11.
                let date = match (*join, c) {
                    ('/', Some(y)) if (1..=12).contains(&a) && (1..=31).contains(&b) => Some((a, b, y)),
                    ('-', Some(d)) if number.len() == 4 && (1..=12).contains(&b) && (1..=31).contains(&d) => Some((b, d, a)),
                    _ => None,
                };
                if let Some((month, day, year)) = date {
                    let mut words = vec![numbers::MONTHS[month as usize - 1].to_string()];
                    words.extend(numbers::ordinal(day));
                    self.say_all(words, span_to(i + 4));
                    self.pause(',', span.0);
                    let year_words = if numbers::is_year(year) { numbers::year(year) } else { numbers::number(t) };
                    self.say_all(year_words, span_to(i + 4));
                    return 5;
                }
            }
            if let (Some(a), Some(b)) = (a, b) {
                let span = span_to(i + 2);
                if *join == '/' && third.is_none() && b > 1 && b <= 10 && a < 100 {
                    // After a whole number, "2 1/2" is "two and a half".
                    let mut words = numbers::fraction(a, b);
                    if self.last_number.is_some_and(|n| n.fract() == 0.0) && a < b {
                        if a == 1 {
                            words[0] = "a".into();
                        }
                        words.insert(0, "and".into());
                    }
                    self.say_all(words, span);
                    return 3;
                }
                if *join != '/' && third.is_none() {
                    let years = number.len() == 4 && second.len() == 4 && numbers::is_year(a) && numbers::is_year(b);
                    if years || (number.len() <= 2 && second.len() <= 2) {
                        let say = |n: u64, len: usize| if len == 4 { numbers::year(n) } else { numbers::cardinal(n) };
                        let mut words = say(a, number.len());
                        words.push("to".into());
                        words.extend(say(b, second.len()));
                        self.say_all(words, span);
                        return 3;
                    }
                }
                // Groups of digits, like a phone number: each digit, with a short pause between groups.
                let mut used = 1;
                let mut n = i;
                while let Part::Number(group, _) = parts[n] {
                    if used > 1 {
                        self.pause(',', span.0);
                    }
                    let words = if group.len() >= 3 { numbers::digits(group) } else { numbers::number(group) };
                    self.say_all(words, span);
                    match (parts.get(n + 1), parts.get(n + 2)) {
                        (Some(Part::Symbol(j, _)), Some(Part::Number(..))) if j == join => {
                            n += 2;
                            used += 2;
                        }
                        _ => break,
                    }
                }
                return used;
            }
        }

        // Letters straight after the number: ordinals, decades, units, and times of day.
        if let Some(&Part::Letters(letters, _)) = next {
            let suffix = plain(letters);
            let whole = numbers::parse(number);
            let span = span_to(i + 1);
            if let Some(n) = whole {
                let ordinal_suffix = match n % 100 {
                    11..=13 => "th",
                    _ => match n % 10 {
                        1 => "st",
                        2 => "nd",
                        3 => "rd",
                        _ => "th",
                    },
                };
                if suffix == ordinal_suffix || (suffix == "th" && matches!(n % 10, 1..=3)) || (matches!(suffix.as_str(), "st" | "nd" | "rd") && n % 100 / 10 == 1) {
                    self.say_all(numbers::ordinal(n), span);
                    return 2;
                }
                if suffix == "s" {
                    let words = if number.len() == 4 && numbers::is_year(n) { numbers::year(n) } else { numbers::cardinal(n) };
                    self.say_all(numbers::plural(words), span);
                    return 2;
                }
            }
            if let Some(&(_, one, many)) = UNITS.iter().find(|u| u.0 == suffix) {
                let mut words = numbers::number(number);
                words.push(if value == Some(1.0) { one } else { many }.to_string());
                self.say_all(words, span);
                return 2;
            }
            if matches!(suffix.as_str(), "am" | "pm" | "a.m" | "p.m") {
                self.say_all(numbers::number(number), parts[i].span());
                self.spell(letters, parts[i + 1].span());
                return 2;
            }
            if letters.chars().all(char::is_uppercase) && letters.chars().count() <= 3 {
                // "4K", "3D".
                self.say_all(numbers::number(number), parts[i].span());
                self.spell(letters, parts[i + 1].span());
                return 2;
            }
        }

        // A percent sign, a day after a month, a year, or a plain number.
        let whole = numbers::parse(number).filter(|_| !number.contains('.'));
        let after_month = matches!(self.pieces.last(), Some(Piece::Word(w)) if numbers::MONTHS.contains(&w.text.as_str()));
        if let Some(day) = whole.filter(|&n| after_month && (1..=31).contains(&n) && next.is_none()) {
            self.say_all(numbers::ordinal(day), span);
            return 1;
        }
        let words = match whole {
            Some(n) if number.len() == 4 && numbers::is_year(n) && !matches!(next, Some(Part::Symbol('%', _))) => {
                let unit_follows = self.peek(index).is_some_and(|w| UNITS.iter().any(|u| u.0 == w && is_separate_unit(u.0)));
                if unit_follows { numbers::number(number) } else { numbers::year(n) }
            }
            _ if number.replace([',', '.'], "").len() > 18 => numbers::digits(number),
            _ => numbers::number(number),
        };
        self.say_number(words, value.unwrap_or(0.0), span);
        1
    }
}

/// Reads `text` into the words to say and the pauses between them.
pub fn normalize(text: &str) -> Vec<Piece> {
    let reader = Reader { chunks: chunks(text), pieces: Vec::new(), last_number: None, currency: None };
    reader.read()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The words and pauses, as one string. Spelled letters are in capitals.
    fn said(text: &str) -> String {
        normalize(text)
            .iter()
            .map(|p| match p {
                Piece::Word(w) if w.spell => w.text.to_uppercase(),
                Piece::Word(w) => w.text.clone(),
                Piece::Break { mark, .. } => mark.to_string(),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn words_and_punctuation() {
        assert_eq!(said("Hello, world!"), "hello , world !");
        assert_eq!(said("“Don’t,” she said... Really?"), "don't , she said . really ?");
        assert_eq!(said("Wait (not yet) please; go."), "wait , not yet , please , go .");
        assert_eq!(said("well-known word—another"), "well known word , another");
        assert_eq!(said("Café naïve"), "cafe naive");
        assert_eq!(said("the dogs' bowls"), "the dogs bowls");
    }

    #[test]
    fn numbers() {
        assert_eq!(said("I have 3 cats and 1,250 fish."), "i have three cats and one thousand two hundred fifty fish .");
        assert_eq!(said("Pi is 3.14"), "pi is three point one four");
        assert_eq!(said("It is -5 out"), "it is minus five out");
        assert_eq!(said("the 21st and 3rd"), "the twenty first and third");
        assert_eq!(said("the 1980s"), "the nineteen eighties");
        assert_eq!(said("in 1984 and 2026"), "in nineteen eighty four and twenty twenty six");
        assert_eq!(said("1500 meters"), "fifteen hundred meters");
        assert_eq!(said("2500 km"), "two thousand five hundred kilometers");
    }

    #[test]
    fn money_and_percent() {
        assert_eq!(said("$5.50"), "five dollars and fifty cents");
        assert_eq!(said("$1"), "one dollar");
        assert_eq!(said("$0.99"), "ninety nine cents");
        assert_eq!(said("$2.5 million"), "two point five million dollars");
        assert_eq!(said("€20"), "twenty euros");
        assert_eq!(said("50%"), "fifty percent");
    }

    #[test]
    fn times_dates_and_ranges() {
        assert_eq!(said("at 3:30"), "at three thirty");
        assert_eq!(said("at 7:00."), "at seven o'clock .");
        assert_eq!(said("10:05 PM"), "ten oh five P M");
        assert_eq!(said("11/11/2026"), "november eleventh , twenty twenty six");
        assert_eq!(said("2026-11-11"), "november eleventh , twenty twenty six");
        assert_eq!(said("pages 5-10"), "pages five to ten");
        assert_eq!(said("1990-1995"), "nineteen ninety to nineteen ninety five");
        assert_eq!(said("555-1234"), "five five five , one two three four");
        assert_eq!(said("3/4 cup"), "three quarters cup");
        assert_eq!(said("on Nov. 11, 2026"), "on november eleventh , twenty twenty six");
        assert_eq!(said("May 5"), "may fifth");
    }

    #[test]
    fn roman_numerals() {
        assert_eq!(said("World War II ended."), "world war two ended .");
        assert_eq!(said("Chapter IV"), "chapter four");
        assert_eq!(said("King Charles III spoke."), "king charles the third spoke .");
        assert_eq!(said("Henry V"), "henry the fifth");
        // Not after a name or a numbering word, and not other capitals.
        assert_eq!(said("I said XIV"), "i said xiv");
        assert_eq!(said("Charles MIX"), "charles mix");
    }

    #[test]
    fn ratios_fractions_and_temperatures() {
        assert_eq!(said("3:1"), "three to one");
        assert_eq!(said("2 1/2 cups"), "two and a half cups");
        assert_eq!(said("1 3/4"), "one and three quarters");
        assert_eq!(said("-5°F"), "minus five degrees fahrenheit");
        assert_eq!(said("20°C"), "twenty degrees celsius");
        assert_eq!(said("3 hrs 15 min"), "three hours fifteen minutes");
    }

    #[test]
    fn units() {
        assert_eq!(said("5km and 1 kg"), "five kilometers and one kilogram");
        assert_eq!(said("60 mph"), "sixty miles per hour");
        assert_eq!(said("2x faster"), "two times faster");
        assert_eq!(said("a 4K screen"), "a four K screen");
    }

    #[test]
    fn initials_and_drive() {
        assert_eq!(said("the U.S. Army"), "the U S army");
        assert_eq!(said("in the U.S. The next day"), "in the U S . the next day");
        assert_eq!(said("Oak Dr. near here"), "oak drive near here");
        assert_eq!(said("Dr. Smith"), "doctor smith");
        assert_eq!(said("on Oak Dr."), "on oak drive .");
    }

    #[test]
    fn abbreviations() {
        assert_eq!(said("Dr. Smith met Mr. Jones."), "doctor smith met mister jones .");
        assert_eq!(said("St. Louis is on Main St. Today"), "saint louis is on main street . today");
        assert_eq!(said("apples, pears, etc. and more"), "apples , pears , et cetera and more");
        assert_eq!(said("fruit, e.g. apples"), "fruit , for example apples");
        assert_eq!(said("cats vs dogs"), "cats versus dogs");
        assert_eq!(said("No. 5"), "number five");
        assert_eq!(said("No. I won't."), "no . i won't .");
    }

    #[test]
    fn letters_and_initials() {
        assert_eq!(said("the U.S. and J. K. Rowling"), "the U S and J K rowling");
        assert_eq!(said("at 9 a.m. sharp"), "at nine A M sharp");
        assert_eq!(said("NASA and the FBI"), "nasa and the fbi");
        assert!(normalize("NASA")[0] == Piece::Word(Word { text: "nasa".into(), offset: 0, len: 4, spell: false, caps: true }));
    }

    #[test]
    fn symbols_and_addresses() {
        assert_eq!(said("salt & pepper"), "salt and pepper");
        assert_eq!(said("2 + 2 = 4"), "two plus two equals four");
        assert_eq!(said("#1"), "number one");
        assert_eq!(said("see https://www.vocalbin.com/vowel-lab"), "see www dot vocalbin dot com slash vowel dash lab");
        assert_eq!(said("mail me@example.com"), "mail me at example dot com");
    }

    #[test]
    fn words_keep_their_place() {
        let pieces = normalize("Hi $5!");
        assert_eq!(pieces[0], Piece::Word(Word { text: "hi".into(), offset: 0, len: 2, spell: false, caps: false }));
        let Piece::Word(five) = &pieces[1] else { panic!() };
        let Piece::Word(dollars) = &pieces[2] else { panic!() };
        assert_eq!((five.offset, five.len, dollars.offset, dollars.len), (3, 2, 3, 2));
        assert_eq!(pieces[3], Piece::Break { mark: '!', offset: 5 });
    }
}
