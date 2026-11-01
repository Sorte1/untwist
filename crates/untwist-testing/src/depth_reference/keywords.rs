use super::escape;
use super::lexer::Tok;

#[derive(Clone, Copy)]
struct Keyword {
    head: u64,
    word: &'static [u8],
    tok: Tok,
}

const KEYWORDS: [(&[u8], Tok); 43] = [
    (b"if", Tok::KwIf),
    (b"in", Tok::KwIn),
    (b"do", Tok::KwDo),
    (b"of", Tok::NameOf),
    (b"for", Tok::KwFor),
    (b"new", Tok::KwNew),
    (b"try", Tok::KwTry),
    (b"var", Tok::KwVar),
    (b"let", Tok::NameLet),
    (b"case", Tok::KwCase),
    (b"else", Tok::KwElse),
    (b"null", Tok::KwNull),
    (b"this", Tok::KwThis),
    (b"true", Tok::KwTrue),
    (b"void", Tok::KwVoid),
    (b"with", Tok::KwWith),
    (b"enum", Tok::NameEnum),
    (b"async", Tok::NameAsync),
    (b"await", Tok::NameAwait),
    (b"yield", Tok::NameYield),
    (b"using", Tok::NameUsing),
    (b"break", Tok::KwBreak),
    (b"catch", Tok::KwCatch),
    (b"class", Tok::KwClass),
    (b"const", Tok::KwConst),
    (b"false", Tok::KwFalse),
    (b"super", Tok::KwSuper),
    (b"throw", Tok::KwThrow),
    (b"while", Tok::KwWhile),
    (b"delete", Tok::KwDelete),
    (b"export", Tok::KwExport),
    (b"import", Tok::KwImport),
    (b"return", Tok::KwReturn),
    (b"switch", Tok::KwSwitch),
    (b"static", Tok::NameStatic),
    (b"typeof", Tok::KwTypeof),
    (b"default", Tok::KwDefault),
    (b"extends", Tok::KwExtends),
    (b"finally", Tok::KwFinally),
    (b"continue", Tok::KwContinue),
    (b"debugger", Tok::KwDebugger),
    (b"function", Tok::KwFunction),
    (b"instanceof", Tok::KwInstanceof),
];

static KEYWORD_TABLE: [Keyword; 128] = build_keywords();

static KEYWORD_LENGTHS: [u16; 256] = build_keyword_lengths();

const fn build_keyword_lengths() -> [u16; 256] {
    let mut table = [0u16; 256];
    let mut index = 0;
    while index < KEYWORDS.len() {
        let word = KEYWORDS[index].0;
        table[word[0] as usize] |= 1 << word.len();
        index += 1;
    }
    table
}

const fn keyword_slot(first: u8, second: u8, last: u8, length: usize) -> usize {
    (first as usize + second as usize + last as usize * 30 + length) & 127
}

const fn head_of(word: &[u8]) -> u64 {
    let mut head = 0u64;
    let mut index = 0;
    while index < word.len() && index < 8 {
        head |= (word[index] as u64) << (8 * index);
        index += 1;
    }
    head
}

const fn build_keywords() -> [Keyword; 128] {
    let mut table = [Keyword {
        head: 0,
        word: b"",
        tok: Tok::Name,
    }; 128];
    let mut index = 0;
    while index < KEYWORDS.len() {
        let (word, tok) = KEYWORDS[index];
        let slot = keyword_slot(word[0], word[1], word[word.len() - 1], word.len());
        table[slot] = Keyword {
            head: head_of(word),
            word,
            tok,
        };
        index += 1;
    }
    table
}

pub(super) fn classify_word(word: &[u8]) -> Tok {
    classify(word, 0, word.len())
}

#[cold]
#[inline(never)]
pub(super) fn classify_escaped(word: &[u8]) -> Tok {
    if !word.contains(&b'\\') {
        return classify_word(word);
    }
    let mut decoded = [0u8; 12];
    match escape::unescaped(word, &mut decoded) {
        Some(length) => classify_word(&decoded[..length]),
        None => Tok::Name,
    }
}

#[inline(always)]
pub(super) fn classify(source: &[u8], start: usize, end: usize) -> Tok {
    let length = end - start;
    let Some(&first) = source.get(start) else {
        return Tok::Name;
    };
    if length > 10 || (KEYWORD_LENGTHS[usize::from(first)] >> length) & 1 == 0 {
        return Tok::Name;
    }
    let (Some(&second), Some(&last)) = (source.get(start + 1), source.get(end - 1)) else {
        return Tok::Name;
    };
    let entry = &KEYWORD_TABLE[keyword_slot(first, second, last, length)];
    if entry.word.len() != length {
        return Tok::Name;
    }
    let matched = match source.get(start..start + 8) {
        Some(chunk) if length <= 8 => {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(chunk);
            u64::from_le_bytes(bytes) & (u64::MAX >> (64 - 8 * length)) == entry.head
        }
        _ => source.get(start..end) == Some(entry.word),
    };
    if matched {
        entry.tok
    } else {
        Tok::Name
    }
}
