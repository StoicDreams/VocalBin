//! Numbers as English words, the American way: "one hundred twenty-three" (no "and"), years in pairs ("nineteen
//! eighty-four"), and ordinals ("twenty-first"). Hyphenated numbers come back as separate words.

const ONES: [&str; 20] = [
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve", "thirteen",
    "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen",
];
const TENS: [&str; 10] = ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];
const SCALES: [(u64, &str); 5] = [
    (1_000_000_000_000_000, "quadrillion"),
    (1_000_000_000_000, "trillion"),
    (1_000_000_000, "billion"),
    (1_000_000, "million"),
    (1_000, "thousand"),
];

pub const MONTHS: [&str; 12] = [
    "january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december",
];

/// The largest number read as a whole. Longer runs of digits are read one digit at a time.
pub const LARGEST: u64 = 999_999_999_999_999_999;

fn below_thousand(n: u64, words: &mut Vec<String>) {
    let (hundreds, rest) = (n / 100, n % 100);
    if hundreds > 0 {
        words.push(ONES[hundreds as usize].into());
        words.push("hundred".into());
    }
    if rest >= 20 {
        words.push(TENS[(rest / 10) as usize].into());
        if rest % 10 > 0 {
            words.push(ONES[(rest % 10) as usize].into());
        }
    } else if rest > 0 || hundreds == 0 {
        words.push(ONES[rest as usize].into());
    }
}

/// `123` as "one hundred twenty three".
pub fn cardinal(n: u64) -> Vec<String> {
    let mut words = Vec::new();
    let mut rest = n;
    for (scale, name) in SCALES {
        if rest >= scale {
            below_thousand(rest / scale, &mut words);
            words.push(name.into());
            rest %= scale;
        }
    }
    if rest > 0 || words.is_empty() {
        below_thousand(rest, &mut words);
    }
    words
}

/// The ordinal form of a number word: "one" to "first", "twenty" to "twentieth".
fn ordinal_word(word: &str) -> String {
    match word {
        "one" => "first".into(),
        "two" => "second".into(),
        "three" => "third".into(),
        "five" => "fifth".into(),
        "eight" => "eighth".into(),
        "nine" => "ninth".into(),
        "twelve" => "twelfth".into(),
        w if w.ends_with('y') => format!("{}ieth", &w[..w.len() - 1]),
        w => format!("{w}th"),
    }
}

/// `21` as "twenty first".
pub fn ordinal(n: u64) -> Vec<String> {
    let mut words = cardinal(n);
    if let Some(last) = words.last_mut() {
        *last = ordinal_word(last);
    }
    words
}

/// The plural of a number's last word, for decades and counts: "nineteen eighties", "twos".
pub fn plural(mut words: Vec<String>) -> Vec<String> {
    if let Some(last) = words.last_mut() {
        *last = match last.as_str() {
            "six" => "sixes".into(),
            w if w.ends_with('y') => format!("{}ies", &w[..w.len() - 1]),
            w => format!("{w}s"),
        };
    }
    words
}

/// Whether a four-digit number reads as a year: 1100 to 1999, and 2000 to 2099.
pub fn is_year(n: u64) -> bool {
    (1100..=2099).contains(&n)
}

/// A year the way people say it: 1984 "nineteen eighty four", 1900 "nineteen hundred", 2005 "two thousand five",
/// 2026 "twenty twenty six".
pub fn year(n: u64) -> Vec<String> {
    let (high, low) = (n / 100, n % 100);
    if (2000..=2009).contains(&n) || n.is_multiple_of(1000) {
        return cardinal(n);
    }
    let mut words = cardinal(high);
    match low {
        0 => words.push("hundred".into()),
        1..=9 => {
            words.push("oh".into());
            words.extend(cardinal(low));
        }
        _ => words.extend(cardinal(low)),
    }
    words
}

/// Each digit on its own, as for phone numbers: "five five five".
pub fn digits(text: &str) -> Vec<String> {
    text.bytes().filter(u8::is_ascii_digit).map(|d| ONES[(d - b'0') as usize].to_string()).collect()
}

/// A whole number written with optional thousands commas, such as `1,234`. `None` if it isn't one, or is too large.
pub fn parse(text: &str) -> Option<u64> {
    let plain: String = text.chars().filter(|&c| c != ',').collect();
    if plain.is_empty() || !plain.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if text.contains(',') {
        // Commas must sit every three digits.
        let groups: Vec<&str> = text.split(',').collect();
        if groups[0].is_empty() || groups[0].len() > 3 || groups[1..].iter().any(|g| g.len() != 3) {
            return None;
        }
    }
    plain.parse::<u64>().ok().filter(|&n| n <= LARGEST)
}

/// A number with an optional decimal part, such as `3.14` "three point one four". Leading zeros and very long
/// numbers are read digit by digit.
pub fn number(text: &str) -> Vec<String> {
    let (whole, fraction) = match text.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (text, None),
    };
    let mut words = if whole.is_empty() {
        Vec::new()
    } else if whole.len() > 1 && whole.starts_with('0') {
        digits(whole)
    } else {
        parse(whole).map_or_else(|| digits(whole), cardinal)
    };
    if let Some(fraction) = fraction {
        words.push("point".into());
        words.extend(digits(fraction));
    }
    words
}

/// The word for one part in `denominator`, for fractions: "half", "third", "quarter", "fifth". Plural when the
/// numerator isn't one.
pub fn fraction(numerator: u64, denominator: u64) -> Vec<String> {
    let mut words = cardinal(numerator);
    let mut part = match denominator {
        2 => vec!["half".to_string()],
        4 => vec!["quarter".to_string()],
        d => ordinal(d),
    };
    if numerator != 1 {
        let last = part.pop().unwrap_or_default();
        part.push(if last == "half" { "halves".into() } else { format!("{last}s") });
    }
    words.extend(part);
    words
}

/// A time such as `3:05` "three oh five", `3:00` "three o'clock", or `15:30` "fifteen thirty".
pub fn time(hours: u64, minutes: u64, with_am_pm: bool) -> Vec<String> {
    let mut words = cardinal(hours);
    match minutes {
        0 if !with_am_pm => words.push("o'clock".into()),
        0 => {}
        1..=9 => {
            words.push("oh".into());
            words.extend(cardinal(minutes));
        }
        _ => words.extend(cardinal(minutes)),
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    fn say(words: Vec<String>) -> String {
        words.join(" ")
    }

    #[test]
    fn cardinals() {
        assert_eq!(say(cardinal(0)), "zero");
        assert_eq!(say(cardinal(7)), "seven");
        assert_eq!(say(cardinal(42)), "forty two");
        assert_eq!(say(cardinal(100)), "one hundred");
        assert_eq!(say(cardinal(123)), "one hundred twenty three");
        assert_eq!(say(cardinal(1_005)), "one thousand five");
        assert_eq!(say(cardinal(2_000_310)), "two million three hundred ten");
        assert_eq!(say(cardinal(LARGEST)).split(' ').next(), Some("nine"));
    }

    #[test]
    fn ordinals() {
        assert_eq!(say(ordinal(1)), "first");
        assert_eq!(say(ordinal(12)), "twelfth");
        assert_eq!(say(ordinal(21)), "twenty first");
        assert_eq!(say(ordinal(40)), "fortieth");
        assert_eq!(say(ordinal(100)), "one hundredth");
    }

    #[test]
    fn years() {
        assert_eq!(say(year(1984)), "nineteen eighty four");
        assert_eq!(say(year(1900)), "nineteen hundred");
        assert_eq!(say(year(1905)), "nineteen oh five");
        assert_eq!(say(year(2000)), "two thousand");
        assert_eq!(say(year(2007)), "two thousand seven");
        assert_eq!(say(year(2026)), "twenty twenty six");
        assert!(is_year(1066 + 100) && !is_year(2100) && !is_year(999));
    }

    #[test]
    fn decimals_and_digits() {
        assert_eq!(say(number("3.14")), "three point one four");
        assert_eq!(say(number(".5")), "point five");
        assert_eq!(say(number("007")), "zero zero seven");
        assert_eq!(say(number("1,234")), "one thousand two hundred thirty four");
        assert_eq!(parse("12,34"), None);
        assert_eq!(parse("1,234,567"), Some(1_234_567));
    }

    #[test]
    fn fractions_and_plurals() {
        assert_eq!(say(fraction(1, 2)), "one half");
        assert_eq!(say(fraction(3, 4)), "three quarters");
        assert_eq!(say(fraction(2, 3)), "two thirds");
        assert_eq!(say(plural(year(1980))), "nineteen eighties");
    }

    #[test]
    fn times() {
        assert_eq!(say(time(3, 5, false)), "three oh five");
        assert_eq!(say(time(3, 0, false)), "three o'clock");
        assert_eq!(say(time(3, 0, true)), "three");
        assert_eq!(say(time(10, 45, true)), "ten forty five");
    }
}
