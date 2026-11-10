use std::borrow::Cow;

use oxc_sourcemap::{OwnedSourceMap, Token};
use rustc_hash::FxHashMap;

use super::lines::Positions;

#[derive(Default)]
struct Interned {
    ids: FxHashMap<u32, u32>,
    texts: Vec<Cow<'static, str>>,
}

impl Interned {
    fn id(&mut self, earlier: u32, text: impl FnOnce(u32) -> Option<String>) -> Option<u32> {
        if let Some(id) = self.ids.get(&earlier) {
            return Some(*id);
        }
        let id = u32::try_from(self.texts.len()).ok()?;
        self.texts.push(Cow::Owned(text(earlier)?));
        self.ids.insert(earlier, id);
        Some(id)
    }
}

pub struct Through<'m> {
    map: &'m OwnedSourceMap,
    table: Vec<&'m [Token]>,
    positions: Positions<'m>,
}

impl<'m> Through<'m> {
    pub fn new(map: &'m OwnedSourceMap, text: &'m str) -> Self {
        Self {
            map,
            table: map.generate_lookup_table(),
            positions: Positions::new(text),
        }
    }

    fn earlier(&self, token: &Token, (line, column): (u32, u32)) -> Option<Token> {
        token.get_source_id()?;
        let column = match token.get_src_line() {
            0 => column.saturating_add(token.get_src_col()),
            _ => token.get_src_col(),
        };
        let line = line.saturating_add(token.get_src_line());
        self.map
            .lookup_token_approx(&self.table, line, column)
            .filter(|earlier| earlier.get_source_id().is_some())
    }

    pub fn rebased(&self, map: OwnedSourceMap, start: usize) -> OwnedSourceMap {
        let base = self.positions.of(start);
        let mut parts = map.into_parts();
        let (mut sources, mut names) = (Interned::default(), Interned::default());
        let source_of = |id| self.map.get_source(id).map(str::to_string);
        let name_of = |id| self.map.get_name(id).map(str::to_string);
        let tokens: Vec<Token> = parts
            .tokens
            .iter()
            .map(|token| {
                let (line, column) = (token.get_dst_line(), token.get_dst_col());
                let found = self.earlier(token, base).and_then(|earlier| {
                    let source = sources.id(earlier.get_source_id()?, source_of)?;
                    Some((earlier, source))
                });
                match found {
                    Some((earlier, source)) => Token::new(
                        line,
                        column,
                        earlier.get_src_line(),
                        earlier.get_src_col(),
                        Some(source),
                        earlier
                            .get_name_id()
                            .and_then(|name| names.id(name, name_of)),
                    ),
                    None => Token::new(line, column, 0, 0, None, None),
                }
            })
            .collect();
        parts.source_contents = vec![None; sources.texts.len()];
        parts.sources = sources.texts;
        parts.names = names.texts;
        parts.tokens = tokens.into_boxed_slice();
        parts.token_chunks = None;
        OwnedSourceMap::from_parts(parts)
    }
}
