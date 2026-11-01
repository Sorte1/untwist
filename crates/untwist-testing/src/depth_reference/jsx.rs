use super::context::Ctx;
use super::lexer::{self, Tok, CLASS, HIGH, ID_PART, ID_START, LINE, SPACE};
use super::read::Read;
use super::{Goal, Scanner};

impl Scanner<'_> {
    pub(super) fn jsx_child_token(&mut self) -> Read {
        let source = self.source;
        while source
            .get(self.position..)
            .is_some_and(|rest| rest.starts_with(b"<!--"))
            && self.html_comment_in_children()
        {
            self.position = lexer::line_end(source, self.position + 4);
        }
        let start = self.position;
        match source.get(start)? {
            b'<' if self.contexts.expr_allowed => {
                self.position = start + 1;
                Some((Tok::JsxTagStart, start, false))
            }
            b'<' | b'{' => self.code_token_at(false),
            _ => {
                self.position = lexer::find_any(source, start, *b"<{");
                Some((Tok::JsxText, start, false))
            }
        }
    }

    fn html_comment_in_children(&mut self) -> bool {
        if self.goal == Goal::Script {
            return true;
        }
        self.divergent = true;
        false
    }

    pub(super) fn jsx_tag_token(&mut self) -> Read {
        let source = self.source;
        let mut newline = false;
        loop {
            let start = self.position;
            let byte = *source.get(start)?;
            let class = CLASS[usize::from(byte)];
            if class & (SPACE | LINE) != 0 {
                self.position = lexer::spaces_end(source, start, &mut newline);
                continue;
            }
            let is_name =
                class & ID_START != 0 || (class & HIGH != 0 && lexer::is_word_char(source, start));
            if is_name {
                self.position = jsx_word_end(source, start + 1);
                return Some((Tok::JsxName, start, newline));
            }
            return match byte {
                b'>' => {
                    self.position = start + 1;
                    Some((Tok::JsxTagEnd, start, newline))
                }
                b'"' | b'\'' if self.contexts.top() == Ctx::JOTag => {
                    let close = lexer::find_any(source, start + 1, [byte]);
                    self.position = (close + 1).min(source.len());
                    Some((Tok::Str, start, newline))
                }
                _ => self.code_token_at(newline),
            };
        }
    }
}

fn jsx_word_end(source: &[u8], mut position: usize) -> usize {
    while let Some(&byte) = source.get(position) {
        if CLASS[usize::from(byte)] & ID_PART != 0 || byte == b'-' {
            position += 1;
        } else if byte >= 0x80 && lexer::is_word_char(source, position) {
            position += lexer::decode(source, position).1;
        } else {
            break;
        }
    }
    position
}
