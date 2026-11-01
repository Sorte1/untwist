use super::escape;

pub(super) const ID_START: u8 = 1;
pub(super) const ID_PART: u8 = 2;
pub(super) const SPACE: u8 = 4;
pub(super) const LINE: u8 = 8;
pub(super) const HIGH: u8 = 16;
pub(super) const BACKSLASH: u8 = 32;

pub(super) static CLASS: [u8; 256] = build_class();

const fn is_space_lead(byte: u8) -> bool {
    matches!(byte, 0xc2 | 0xe1 | 0xe2 | 0xe3 | 0xef)
}

const fn build_class() -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut index = 0;
    while index < 256 {
        let byte = index as u8;
        table[index] = if byte.is_ascii_alphabetic() || byte == b'$' || byte == b'_' {
            ID_START | ID_PART
        } else if byte.is_ascii_digit() {
            ID_PART
        } else if byte == b' ' || byte == b'\t' || byte == 0x0b || byte == 0x0c {
            SPACE
        } else if byte == b'\n' || byte == b'\r' {
            LINE
        } else if is_space_lead(byte) {
            HIGH
        } else if byte >= 0x80 {
            ID_START | ID_PART
        } else if byte == b'\\' {
            BACKSLASH
        } else {
            0
        };
        index += 1;
    }
    table
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub(super) enum Tok {
    Start,
    Name,
    NameAsync,
    NameOf,
    NameYield,
    NameAwait,
    NameLet,
    NameStatic,
    NameBinding,
    NameUsing,
    NameEnum,
    KwBreak,
    KwCase,
    KwCatch,
    KwContinue,
    KwDebugger,
    KwDefault,
    KwDo,
    KwElse,
    KwFinally,
    KwFor,
    KwFunction,
    KwIf,
    KwReturn,
    KwSwitch,
    KwThrow,
    KwTry,
    KwVar,
    KwConst,
    KwWhile,
    KwWith,
    KwNew,
    KwThis,
    KwSuper,
    KwClass,
    KwExtends,
    KwExport,
    KwImport,
    KwNull,
    KwTrue,
    KwFalse,
    KwIn,
    KwInstanceof,
    KwTypeof,
    KwVoid,
    KwDelete,
    Num,
    Str,
    Regex,
    Template,
    BackQuote,
    PrivateName,
    ParenL,
    ParenR,
    BracketL,
    BracketR,
    BraceL,
    BraceR,
    DollarBraceL,
    Comma,
    Semi,
    Colon,
    Dot,
    QuestionDot,
    Question,
    Arrow,
    Ellipsis,
    Eq,
    Op,
    Slash,
    Logical,
    IncDec,
    Star,
    At,
    JsxTagStart,
    JsxTagEnd,
    JsxName,
    JsxText,
}

pub(super) const BEFORE_EXPR: u8 = 1;
pub(super) const RUN: u8 = 2;
pub(super) const LINE_TOKEN: u8 = 4;
pub(super) const SPECIAL: u8 = 8;

const TOKEN_COUNT: usize = Tok::JsxText as usize + 1;

pub(super) static FLAGS: [u8; TOKEN_COUNT] = build_flags();

const fn build_flags() -> [u8; TOKEN_COUNT] {
    let mut table = [0u8; TOKEN_COUNT];
    let mut index = 0;
    while index < TOKEN_COUNT {
        table[index] = flags_of(index);
        index += 1;
    }
    table
}

const fn flags_of(index: usize) -> u8 {
    let keyword = index >= Tok::KwBreak as usize && index <= Tok::KwDelete as usize;
    let before = keyword && is_before_expr_keyword(index)
        || (index >= Tok::ParenL as usize
            && index <= Tok::JsxText as usize
            && is_before_expr_punctuator(index));
    let run = keyword
        || index == Tok::NameYield as usize
        || index == Tok::NameAwait as usize
        || (index >= Tok::ParenL as usize && index <= Tok::JsxTagEnd as usize);
    let line = is_line_token(index);
    let special = is_special(index);
    (before as u8) * BEFORE_EXPR
        + (run as u8) * RUN
        + (line as u8) * LINE_TOKEN
        + (special as u8) * SPECIAL
}

const fn is_before_expr_keyword(index: usize) -> bool {
    index == Tok::KwCase as usize
        || index == Tok::KwDefault as usize
        || index == Tok::KwDo as usize
        || index == Tok::KwElse as usize
        || index == Tok::KwReturn as usize
        || index == Tok::KwThrow as usize
        || index == Tok::KwNew as usize
        || index == Tok::KwExtends as usize
        || index >= Tok::KwIn as usize && index <= Tok::KwDelete as usize
}

const fn is_before_expr_punctuator(index: usize) -> bool {
    index == Tok::ParenL as usize
        || index == Tok::BracketL as usize
        || index == Tok::BraceL as usize
        || index == Tok::DollarBraceL as usize
        || index == Tok::Comma as usize
        || index == Tok::Semi as usize
        || index == Tok::Colon as usize
        || index == Tok::Question as usize
        || index == Tok::Arrow as usize
        || index == Tok::Ellipsis as usize
        || index == Tok::Eq as usize
        || index == Tok::Op as usize
        || index == Tok::Slash as usize
        || index == Tok::Logical as usize
        || index == Tok::Star as usize
        || index == Tok::JsxText as usize
        || index == Tok::At as usize
}

const fn is_line_token(index: usize) -> bool {
    let keyword_line = index >= Tok::KwBreak as usize
        && index <= Tok::KwClass as usize
        && index != Tok::KwDebugger as usize
        && index != Tok::KwNew as usize
        && index != Tok::KwThis as usize
        && index != Tok::KwSuper as usize;
    keyword_line
        || index == Tok::NameLet as usize
        || index == Tok::Semi as usize
        || index == Tok::Comma as usize
        || index == Tok::BraceL as usize
        || index == Tok::BraceR as usize
        || index == Tok::ParenR as usize
        || index == Tok::Question as usize
        || index == Tok::Colon as usize
        || index == Tok::Arrow as usize
        || index == Tok::Logical as usize
}

const fn is_special(index: usize) -> bool {
    (index >= Tok::ParenL as usize && index <= Tok::DollarBraceL as usize)
        || index == Tok::Comma as usize
        || index == Tok::Semi as usize
        || index == Tok::Colon as usize
        || index == Tok::Question as usize
        || index == Tok::Arrow as usize
        || index == Tok::IncDec as usize
        || index == Tok::Star as usize
        || index == Tok::BackQuote as usize
        || index == Tok::At as usize
        || index == Tok::NameOf as usize
        || index == Tok::NameYield as usize
        || index == Tok::NameAwait as usize
        || index == Tok::NameAsync as usize
        || index == Tok::NameLet as usize
        || index == Tok::NameUsing as usize
        || index == Tok::KwCase as usize
        || index == Tok::KwDefault as usize
        || index == Tok::KwFunction as usize
        || index == Tok::KwClass as usize
        || index == Tok::KwIf as usize
        || index == Tok::KwFor as usize
        || index == Tok::KwWhile as usize
        || index == Tok::KwWith as usize
        || index == Tok::KwDo as usize
        || index == Tok::KwElse as usize
        || index == Tok::KwBreak as usize
        || index == Tok::KwContinue as usize
        || index == Tok::KwDebugger as usize
        || index == Tok::KwImport as usize
        || index == Tok::KwExport as usize
        || index >= Tok::JsxTagStart as usize && index <= Tok::JsxTagEnd as usize
}

impl Tok {
    #[inline(always)]
    pub(super) fn flags(self) -> u8 {
        FLAGS[self as usize]
    }

    pub(super) fn before_expr(self) -> bool {
        self.flags() & BEFORE_EXPR != 0
    }

    pub(super) fn is_name(self) -> bool {
        (Tok::Name as u8..=Tok::NameEnum as u8).contains(&(self as u8))
    }

    #[inline(always)]
    pub(super) fn declares(self) -> bool {
        matches!(self, Tok::KwVar | Tok::KwConst | Tok::NameLet)
    }

    pub(super) fn is_literal(self) -> bool {
        matches!(
            self,
            Tok::Num | Tok::Str | Tok::Regex | Tok::KwNull | Tok::KwTrue | Tok::KwFalse
        )
    }
}

const OXC_WORDS: &[&[u8]] = &[
    b"as",
    b"is",
    b"any",
    b"get",
    b"out",
    b"set",
    b"from",
    b"meta",
    b"type",
    b"infer",
    b"keyof",
    b"never",
    b"using",
    b"defer",
    b"assert",
    b"bigint",
    b"global",
    b"module",
    b"number",
    b"object",
    b"public",
    b"static",
    b"string",
    b"symbol",
    b"target",
    b"unique",
    b"source",
    b"asserts",
    b"boolean",
    b"declare",
    b"package",
    b"private",
    b"require",
    b"unknown",
    b"abstract",
    b"accessor",
    b"override",
    b"readonly",
    b"interface",
    b"intrinsic",
    b"namespace",
    b"protected",
    b"satisfies",
    b"undefined",
    b"implements",
    b"constructor",
];

pub(super) fn is_plain_oxc_identifier(word: &[u8]) -> bool {
    !word.contains(&b'\\') && !OXC_WORDS.contains(&word)
}

pub(super) fn decode(source: &[u8], position: usize) -> (u32, usize) {
    let lead = source[position];
    let (width, mut code) = match lead {
        0xc0..=0xdf => (2, u32::from(lead & 0x1f)),
        0xe0..=0xef => (3, u32::from(lead & 0x0f)),
        0xf0..=0xf7 => (4, u32::from(lead & 0x07)),
        _ => return (u32::from(lead), 1),
    };
    let end = (position + width).min(source.len());
    for byte in &source[position + 1..end] {
        code = (code << 6) | u32::from(byte & 0x3f);
    }
    (code, end - position)
}

pub(super) fn is_unicode_space(code: u32) -> bool {
    matches!(
        code,
        0xa0 | 0x1680 | 0x2000..=0x200a | 0x202f | 0x205f | 0x3000 | 0xfeff
    )
}

pub(super) fn is_unicode_line(code: u32) -> bool {
    code == 0x2028 || code == 0x2029
}

const LOW_BITS: u64 = 0x0101_0101_0101_0101;
const HIGH_BITS: u64 = 0x8080_8080_8080_8080;

#[inline(always)]
fn zero_bytes(word: u64) -> u64 {
    word.wrapping_sub(LOW_BITS) & !word & HIGH_BITS
}

#[inline(always)]
pub(super) fn find_any<const N: usize>(
    source: &[u8],
    mut position: usize,
    needles: [u8; N],
) -> usize {
    while let Some(chunk) = source.get(position..position + 8) {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(chunk);
        let word = u64::from_le_bytes(bytes);
        let mut hits = 0;
        for needle in needles {
            hits |= zero_bytes(word ^ (LOW_BITS * u64::from(needle)));
        }
        if hits != 0 {
            return position + (hits.trailing_zeros() / 8) as usize;
        }
        position += 8;
    }
    while position < source.len() && !needles.contains(&source[position]) {
        position += 1;
    }
    position
}

pub(super) fn line_end(source: &[u8], mut position: usize) -> usize {
    loop {
        position = find_any(source, position, [b'\n', b'\r', 0xe2]);
        if position >= source.len() || source[position] != 0xe2 {
            return position;
        }
        if is_unicode_line(decode(source, position).0) {
            return position;
        }
        position += 1;
    }
}

pub(super) fn block_comment_end(source: &[u8], mut position: usize) -> (usize, bool) {
    let mut newline = false;
    loop {
        position = find_any(source, position, [b'*', b'\n', b'\r', 0xe2]);
        let Some(&byte) = source.get(position) else {
            return (position, newline);
        };
        if byte == b'*' && source.get(position + 1) == Some(&b'/') {
            return (position + 2, newline);
        }
        newline |= byte == b'\n' || byte == b'\r';
        newline |= byte == 0xe2 && is_unicode_line(decode(source, position).0);
        position += 1;
    }
}

#[inline(always)]
fn chunk(source: &[u8], position: usize) -> Option<u64> {
    let bytes: [u8; 8] = source.get(position..position + 8)?.try_into().ok()?;
    Some(u64::from_le_bytes(bytes))
}

#[inline(always)]
fn string_stop(source: &[u8], mut position: usize, quote: u8) -> usize {
    let quotes = LOW_BITS * u64::from(quote);
    let slashes = LOW_BITS * u64::from(b'\\');
    while let Some(word) = chunk(source, position) {
        let controls = word.wrapping_sub(LOW_BITS * 0x0e) & !word & HIGH_BITS;
        let hits = zero_bytes(word ^ quotes) | zero_bytes(word ^ slashes) | controls;
        if hits != 0 {
            return position + (hits.trailing_zeros() / 8) as usize;
        }
        position += 8;
    }
    while let Some(&byte) = source.get(position) {
        if byte == quote || byte == b'\\' || byte < 0x0e {
            break;
        }
        position += 1;
    }
    position
}

pub(super) fn string_end(source: &[u8], mut position: usize, quote: u8) -> usize {
    loop {
        position = string_stop(source, position, quote);
        match source.get(position) {
            None => return position,
            Some(&b'\\') => position = skip_escape(source, position),
            Some(&byte) if byte == quote => return position + 1,
            Some(&b'\n' | &b'\r') => return position,
            Some(_) => position += 1,
        }
    }
}

fn skip_escape(source: &[u8], position: usize) -> usize {
    let next = position + 1;
    if source.get(next) == Some(&b'\r') && source.get(next + 1) == Some(&b'\n') {
        return next + 2;
    }
    match source.get(next) {
        Some(&byte) if byte >= 0x80 => next + decode(source, next).1,
        Some(_) => next + 1,
        None => next,
    }
}

pub(super) fn template_chunk_end(source: &[u8], mut position: usize) -> usize {
    loop {
        position = find_any(source, position, *b"`$\\");
        match source.get(position) {
            None | Some(&b'`') => return position,
            Some(&b'\\') => position = skip_escape(source, position),
            Some(_) if source.get(position + 1) == Some(&b'{') => return position,
            Some(_) => position += 1,
        }
    }
}

fn flags_end(source: &[u8], mut position: usize) -> usize {
    while source
        .get(position)
        .is_some_and(|&byte| byte.is_ascii_alphanumeric() || matches!(byte, b'$' | b'_'))
    {
        position += 1;
    }
    position
}

pub(super) fn regex_end(source: &[u8], mut position: usize) -> Option<usize> {
    let mut in_class = false;
    while let Some(&byte) = source.get(position) {
        match byte {
            b'\n' | b'\r' => return None,
            0xe2 if is_unicode_line(decode(source, position).0) => return None,
            b'\\' => {
                if matches!(source.get(position + 1), None | Some(b'\n' | b'\r')) {
                    return None;
                }
                position = skip_escape(source, position);
                continue;
            }
            b'[' => in_class = true,
            b']' => in_class = false,
            b'/' if !in_class => return Some(flags_end(source, position + 1)),
            _ => {}
        }
        position += 1;
    }
    None
}

#[inline(always)]
fn eight(source: &[u8], position: usize) -> Option<&[u8; 8]> {
    source.get(position..)?.first_chunk::<8>()
}

#[inline(always)]
fn first_outside(bytes: &[u8; 8], class: u8) -> Option<usize> {
    let inside = |index: usize| CLASS[usize::from(bytes[index])] & class != 0;
    if !inside(0) {
        return Some(0);
    }
    if !inside(1) {
        return Some(1);
    }
    if !inside(2) {
        return Some(2);
    }
    if !inside(3) {
        return Some(3);
    }
    if !inside(4) {
        return Some(4);
    }
    if !inside(5) {
        return Some(5);
    }
    if !inside(6) {
        return Some(6);
    }
    if !inside(7) {
        return Some(7);
    }
    None
}

#[inline(always)]
pub(super) fn class_run_end(source: &[u8], mut position: usize, class: u8) -> usize {
    while let Some(bytes) = eight(source, position) {
        if let Some(offset) = first_outside(bytes, class) {
            return position + offset;
        }
        position += 8;
    }
    while let Some(&byte) = source.get(position) {
        if CLASS[usize::from(byte)] & class == 0 {
            break;
        }
        position += 1;
    }
    position
}

#[inline(always)]
fn stop_in_four(bytes: &[u8; 4]) -> Option<(usize, u8)> {
    for (index, &byte) in bytes.iter().enumerate() {
        let class = CLASS[usize::from(byte)];
        if class & ID_PART == 0 {
            return Some((index, class));
        }
    }
    None
}

#[inline(always)]
pub(super) fn word_end(source: &[u8], position: usize) -> usize {
    word_span(source, position).0
}

#[inline(always)]
pub(super) fn word_span(source: &[u8], mut position: usize) -> (usize, bool) {
    let class = loop {
        if let Some(bytes) = source.get(position..).and_then(<[u8]>::first_chunk::<4>) {
            if let Some((offset, class)) = stop_in_four(bytes) {
                position += offset;
                break class;
            }
            position += 4;
            continue;
        }
        let Some(&byte) = source.get(position) else {
            break 0;
        };
        let class = CLASS[usize::from(byte)];
        if class & ID_PART == 0 {
            break class;
        }
        position += 1;
    };
    if class & (HIGH | BACKSLASH) != 0 {
        return (word_rest(source, position), true);
    }
    (position, false)
}

#[cold]
#[inline(never)]
pub(super) fn word_rest(source: &[u8], mut position: usize) -> usize {
    loop {
        match source.get(position) {
            Some(&b'\\') => position = escape::escape_end(source, position),
            Some(&byte) if byte >= 0x80 && is_word_char(source, position) => {
                position += decode(source, position).1;
            }
            _ => return position,
        }
        position = class_run_end(source, position, ID_PART);
    }
}

#[inline(always)]
pub(super) fn spaces_end(source: &[u8], mut position: usize, newline: &mut bool) -> usize {
    loop {
        while source.get(position) == Some(&b' ') {
            position += 1;
        }
        if source.get(position) != Some(&b'\n') {
            break;
        }
        *newline = true;
        position += 1;
    }
    while let Some(&byte) = source.get(position) {
        let class = CLASS[usize::from(byte)];
        if class & (SPACE | LINE) == 0 {
            break;
        }
        *newline |= class & LINE != 0;
        position += 1;
    }
    position
}

pub(super) fn is_word_char(source: &[u8], position: usize) -> bool {
    let code = decode(source, position).0;
    !is_unicode_space(code) && !is_unicode_line(code)
}

pub(super) fn number_end(source: &[u8], start: usize) -> usize {
    let radix_prefixed = source[start] == b'0'
        && matches!(
            source.get(start + 1),
            Some(b'x' | b'X' | b'b' | b'B' | b'o' | b'O')
        );
    let mut position = start + 1;
    while let Some(&byte) = source.get(position) {
        if CLASS[usize::from(byte)] & ID_PART == 0
            && byte != b'.'
            && !(matches!(byte, b'+' | b'-')
                && !radix_prefixed
                && matches!(source[position - 1], b'e' | b'E'))
        {
            break;
        }
        position += 1;
    }
    position
}

#[inline(always)]
pub(super) fn punctuator(source: &[u8], position: usize) -> (Tok, usize) {
    let at = |offset: usize| source.get(position + offset).copied().unwrap_or(0);
    match source[position] {
        b'(' => (Tok::ParenL, 1),
        b')' => (Tok::ParenR, 1),
        b'[' => (Tok::BracketL, 1),
        b']' => (Tok::BracketR, 1),
        b'{' => (Tok::BraceL, 1),
        b'}' => (Tok::BraceR, 1),
        b',' => (Tok::Comma, 1),
        b';' => (Tok::Semi, 1),
        b':' => (Tok::Colon, 1),
        b'@' => (Tok::At, 1),
        b'.' if at(1) == b'.' && at(2) == b'.' => (Tok::Ellipsis, 3),
        b'.' => (Tok::Dot, 1),
        b'?' => question(at(1), at(2)),
        b'=' => equals(at(1), at(2)),
        b'+' | b'-' => plus_minus(source[position], at(1)),
        b'*' => star(at(1), at(2)),
        b'&' | b'|' => logical(source[position], at(1), at(2)),
        b'<' | b'>' => (Tok::Op, angle_length(source, position)),
        b'/' if at(1) != b'=' => (Tok::Slash, 1),
        b'!' if at(1) == b'=' => (Tok::Op, 2 + usize::from(at(2) == b'=')),
        b'!' | b'%' | b'^' | b'/' => (Tok::Op, 1 + usize::from(at(1) == b'=')),
        _ => (Tok::Op, 1),
    }
}

fn question(next: u8, after: u8) -> (Tok, usize) {
    match next {
        b'.' if !after.is_ascii_digit() => (Tok::QuestionDot, 2),
        b'?' if after == b'=' => (Tok::Op, 3),
        b'?' => (Tok::Logical, 2),
        _ => (Tok::Question, 1),
    }
}

fn equals(next: u8, after: u8) -> (Tok, usize) {
    match (next, after) {
        (b'>', _) => (Tok::Arrow, 2),
        (b'=', b'=') => (Tok::Op, 3),
        (b'=', _) => (Tok::Op, 2),
        _ => (Tok::Eq, 1),
    }
}

fn plus_minus(byte: u8, next: u8) -> (Tok, usize) {
    if next == byte {
        (Tok::IncDec, 2)
    } else {
        (Tok::Op, 1 + usize::from(next == b'='))
    }
}

fn star(next: u8, after: u8) -> (Tok, usize) {
    match (next, after) {
        (b'*', b'=') => (Tok::Op, 3),
        (b'*', _) | (b'=', _) => (Tok::Op, 2),
        _ => (Tok::Star, 1),
    }
}

fn logical(byte: u8, next: u8, after: u8) -> (Tok, usize) {
    let doubled = next == byte;
    let assigns = if doubled { after == b'=' } else { next == b'=' };
    match (doubled, assigns) {
        (true, true) => (Tok::Op, 3),
        (true, false) => (Tok::Logical, 2),
        (false, true) => (Tok::Op, 2),
        (false, false) => (Tok::Op, 1),
    }
}

fn angle_length(source: &[u8], position: usize) -> usize {
    let byte = source[position];
    let mut length = 1;
    let limit = if byte == b'>' { 3 } else { 2 };
    while length < limit && source.get(position + length) == Some(&byte) {
        length += 1;
    }
    length + usize::from(source.get(position + length) == Some(&b'='))
}
