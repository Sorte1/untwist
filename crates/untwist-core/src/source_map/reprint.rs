use oxc_sourcemap::{OwnedSourceMap, Token};

use super::lines::Positions;

pub struct Reprint<'t> {
    pub map: OwnedSourceMap,
    pub printed: &'t str,
    pub earlier: &'t str,
}

pub type Spot = Option<(u32, u32)>;

const END_PROBES: usize = 64;

struct Step<'s, 't> {
    limit: usize,
    map: &'s OwnedSourceMap,
    table: Vec<&'s [Token]>,
    printed: Positions<'t>,
    earlier: Positions<'t>,
}

impl<'s, 't> Step<'s, 't> {
    fn of(reprint: &'s Reprint<'t>) -> Self {
        Self {
            limit: reprint.printed.len(),
            map: &reprint.map,
            table: reprint.map.generate_lookup_table(),
            printed: Positions::new(reprint.printed),
            earlier: Positions::new(reprint.earlier),
        }
    }

    fn offset(&self, offset: u32) -> Option<u32> {
        let offset = usize::try_from(offset)
            .ok()
            .filter(|at| *at <= self.limit)?;
        let (line, column) = self.printed.of(offset);
        let token = self
            .map
            .lookup_token_approx(&self.table, line, column)
            .filter(|token| token.get_source_id().is_some())?;
        let shift = match token.get_dst_line() == line {
            true => column.saturating_sub(token.get_dst_col()),
            false => 0,
        };
        let column = token.get_src_col().saturating_add(shift);
        let at = self.earlier.offset(token.get_src_line(), column)?;
        u32::try_from(at).ok()
    }

    fn end(&self, (start, end): (u32, u32), from: u32) -> u32 {
        (start..end)
            .rev()
            .take(END_PROBES)
            .filter_map(|at| self.offset(at))
            .find(|last| *last >= from)
            .map_or(from, |last| last.saturating_add(1))
    }

    fn spot(&self, spot: Spot) -> Spot {
        let (start, end) = spot?;
        let from = self.offset(start)?;
        Some((from, self.end((start, end), from)))
    }
}

pub fn spots_back(reprints: &[Reprint<'_>], spots: Vec<Spot>) -> Vec<Spot> {
    reprints.iter().rev().fold(spots, |spots, reprint| {
        let step = Step::of(reprint);
        spots.into_iter().map(|spot| step.spot(spot)).collect()
    })
}

pub fn composed(map: OwnedSourceMap, reprints: &[Reprint<'_>]) -> OwnedSourceMap {
    reprints
        .iter()
        .rev()
        .fold(map, |map, reprint| match map.get_sources().count() > 1 {
            true => map,
            false => map.compose(reprint.map.clone()),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_map::identity;

    #[test]
    fn a_reprint_that_kept_every_line_traces_spans_to_the_same_place() {
        let text = "var a = 1;\nvar b = 2;\n";
        let reprints = [Reprint {
            map: identity(text),
            printed: text,
            earlier: text,
        }];
        let traced = spots_back(&reprints, vec![Some((11, 21)), Some((0, 0)), None]);
        assert_eq!(traced, [Some((11, 21)), Some((0, 0)), None]);
    }

    #[test]
    fn an_offset_past_the_printed_text_is_lost() {
        let text = "a();";
        let reprints = [Reprint {
            map: identity(text),
            printed: text,
            earlier: text,
        }];
        assert_eq!(spots_back(&reprints, vec![Some((9, 12))]), [None]);
    }

    #[test]
    fn a_composed_map_points_into_the_first_text() {
        let first = "x();\ny();\n";
        let reprints = [Reprint {
            map: identity(first),
            printed: first,
            earlier: first,
        }];
        let map = composed(identity(first), &reprints);
        let tokens: Vec<(u32, u32)> = map
            .get_tokens()
            .map(|token| (token.get_dst_line(), token.get_src_line()))
            .collect();
        assert_eq!(tokens, [(0, 0), (1, 1)]);
        assert_eq!(map.get_source_content(0), Some(first));
    }
}
