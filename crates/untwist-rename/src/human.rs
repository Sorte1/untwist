const CONFUSABLES: [char; 21] = [
    'i', 'l', 'I', 'ì', 'í', 'î', 'ï', 'Ì', 'Í', 'Î', 'Ï', 'ı', 'ī', 'ĭ', 'į', 'ɩ', 'Ι', 'І', 'ⅼ',
    'ℓ', '¡',
];

pub fn looks_human_written(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    !(is_single_character(name)
        || is_hex_style(name)
        || is_confusable_run(name)
        || is_short_mixed_case_without_vowel(name))
}

fn is_single_character(name: &str) -> bool {
    name.chars().nth(1).is_none()
}

fn is_hex_style(name: &str) -> bool {
    let stem = name.trim_start_matches('_');
    let Some(digits) = stem.strip_prefix("0x").or_else(|| stem.strip_prefix("0X")) else {
        return false;
    };
    !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn is_confusable_run(name: &str) -> bool {
    let mut count = 0usize;
    for ch in name.chars() {
        if !CONFUSABLES.contains(&ch) {
            return false;
        }
        count += 1;
    }
    count >= 2
}

fn is_short_mixed_case_without_vowel(name: &str) -> bool {
    let length = name.chars().count();
    if !(2..=4).contains(&length) {
        return false;
    }
    let mut upper = false;
    let mut lower = false;
    for ch in name.chars() {
        if is_vowel(ch) {
            return false;
        }
        if ch.is_ascii_uppercase() {
            upper = true;
        }
        if ch.is_ascii_lowercase() {
            lower = true;
        }
    }
    upper && lower
}

fn is_vowel(ch: char) -> bool {
    matches!(
        ch,
        'a' | 'e' | 'i' | 'o' | 'u' | 'A' | 'E' | 'I' | 'O' | 'U'
    )
}
