use oxc_ast::ast::Program;
use oxc_span::Span;
use untwist_core::source_map::{self, Origins, OwnedSourceMap, Reprint, Spot};

const LOST: Span = Span::new(u32::MAX, u32::MAX);

#[derive(Default)]
pub(super) struct Trail<'a> {
    reprints: Vec<Reprint<'a>>,
}

impl<'a> Trail<'a> {
    pub(super) fn is_empty(&self) -> bool {
        self.reprints.is_empty()
    }

    pub(super) fn reprinted(
        &mut self,
        held: &mut Program<'a>,
        origins: Option<Origins>,
        printed: &'a str,
    ) {
        let earlier = held.source_text;
        let map = match source_map::mapped(held, &origins.unwrap_or_default()) {
            Some(mapped) => source_map::retarget(mapped.map, &mapped.printed, printed),
            None => source_map::unmapped(earlier),
        };
        self.reprints.push(Reprint {
            map,
            printed,
            earlier,
        });
    }

    pub(super) fn spots(&self, spots: Vec<Spot>) -> Vec<Spot> {
        match self.reprints.is_empty() {
            true => spots,
            false => source_map::spots_back(&self.reprints, spots),
        }
    }

    pub(super) fn spans(&self, spans: impl Iterator<Item = Span>) -> Vec<Span> {
        let spots = spans.map(|span| Some((span.start, span.end))).collect();
        self.spots(spots)
            .into_iter()
            .map(|spot| spot.map_or(LOST, |(start, end)| Span::new(start, end)))
            .collect()
    }

    pub(super) fn composed(&self, map: OwnedSourceMap) -> OwnedSourceMap {
        source_map::composed(map, &self.reprints)
    }
}

#[derive(Clone, Copy)]
pub(super) struct Taps {
    decoded: usize,
    #[cfg(feature = "refusal-spans")]
    refusals: usize,
}

impl Taps {
    pub(super) fn now() -> Self {
        Self {
            decoded: untwist_deobfuscate::decoded_strings::recorded(),
            #[cfg(feature = "refusal-spans")]
            refusals: untwist_deobfuscate::refusal_spans::recorded(),
        }
    }

    pub(super) fn rewind(self) {
        drop(untwist_deobfuscate::decoded_strings::taken_since(
            self.decoded,
        ));
        #[cfg(feature = "refusal-spans")]
        drop(untwist_deobfuscate::refusal_spans::taken_since(
            self.refusals,
        ));
    }

    pub(super) fn settle(self, trail: &Trail<'_>) {
        if trail.is_empty() {
            return;
        }
        use untwist_deobfuscate::decoded_strings::{put_back, taken_since, DecodedString};
        let found = taken_since(self.decoded);
        let spans = trail.spans(found.iter().map(|found| found.span));
        put_back(
            found
                .into_iter()
                .zip(spans)
                .map(|(found, span)| DecodedString { span, ..found })
                .collect(),
        );
        #[cfg(feature = "refusal-spans")]
        {
            use untwist_deobfuscate::refusal_spans::{put_back, taken_since, RefusalSpan};
            let found = taken_since(self.refusals);
            let spans = trail.spans(found.iter().map(|found| found.span));
            put_back(
                found
                    .into_iter()
                    .zip(spans)
                    .map(|(found, span)| RefusalSpan { span, ..found })
                    .collect(),
            );
        }
    }
}
