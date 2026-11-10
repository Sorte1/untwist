use oxc_allocator::GetAddress;
use oxc_ast::ast::{Expression, Program, Statement};
use oxc_ast::AstType;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_span::Span;

use super::origins::Origins;
use crate::ast_util::respan::SYNTHETIC_BASE;

const NO_PARENT: u32 = u32::MAX;

struct Entry {
    span: Span,
    parent: u32,
}

struct Collect<'o> {
    origins: &'o Origins,
    limit: u32,
    entries: Vec<Entry>,
    frames: Vec<Option<u32>>,
    forced: Vec<Span>,
}

impl Collect<'_> {
    fn is_real(&self, span: Span) -> bool {
        span.start < span.end && span.end <= self.limit
    }

    fn parent(&self) -> u32 {
        self.frames
            .iter()
            .rev()
            .find_map(|frame| *frame)
            .unwrap_or(NO_PARENT)
    }

    fn forced(&self, span: Span) -> Span {
        let span = self.origins.resolved(span);
        match self.forced.last() {
            Some(origin) if !self.is_real(span) || !origin.contains_inclusive(span) => *origin,
            _ => span,
        }
    }

    fn entered(&mut self, address: oxc_allocator::Address) -> bool {
        let origin = self
            .origins
            .replaced(address)
            .map(|span| self.origins.resolved(span));
        match origin {
            Some(span) if self.is_real(span) => {
                self.forced.push(span);
                true
            }
            _ => false,
        }
    }
}

impl<'a> VisitMut<'a> for Collect<'_> {
    fn enter_node(&mut self, _kind: AstType) {
        self.frames.push(None);
    }

    fn leave_node(&mut self, _kind: AstType) {
        self.frames.pop();
    }

    fn visit_span(&mut self, it: &mut Span) {
        let index = u32::try_from(self.entries.len()).unwrap_or(NO_PARENT);
        let parent = self.parent();
        let span = self.forced(*it);
        self.entries.push(Entry { span, parent });
        if let Some(frame @ None) = self.frames.last_mut() {
            *frame = Some(index);
        }
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        let forced = self.entered(it.address());
        walk_mut::walk_expression(self, it);
        if forced {
            self.forced.pop();
        }
    }

    fn visit_statement(&mut self, it: &mut Statement<'a>) {
        let forced = self.entered(it.address());
        walk_mut::walk_statement(self, it);
        if forced {
            self.forced.pop();
        }
    }
}

struct Assign {
    spans: std::vec::IntoIter<Span>,
}

impl<'a> VisitMut<'a> for Assign {
    fn visit_span(&mut self, it: &mut Span) {
        if let Some(span) = self.spans.next() {
            *it = span;
        }
    }
}

fn settled(entries: &[Entry], is_real: impl Fn(Span) -> bool) -> Vec<Span> {
    let mut first: Vec<Option<Span>> = vec![None; entries.len()];
    for (index, entry) in entries.iter().enumerate().rev() {
        let candidate = match is_real(entry.span) {
            true => Some(entry.span),
            false => first[index],
        };
        if let (Some(candidate), Some(slot)) = (candidate, first.get_mut(entry.parent as usize)) {
            *slot = Some(candidate);
        }
    }
    let mut spans: Vec<Span> = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let span = match (is_real(entry.span), first[index]) {
            (true, _) => entry.span,
            (false, Some(found)) => found,
            (false, None) => spans
                .get(entry.parent as usize)
                .copied()
                .filter(|span| is_real(*span))
                .unwrap_or_default(),
        };
        spans.push(span);
    }
    spans
}

fn filled(program: &mut Program<'_>, origins: &Origins) -> Vec<Span> {
    let limit = u32::try_from(program.source_text.len())
        .unwrap_or(u32::MAX)
        .min(SYNTHETIC_BASE);
    let mut collect = Collect {
        origins,
        limit,
        entries: Vec::new(),
        frames: Vec::new(),
        forced: Vec::new(),
    };
    program.span = Span::default();
    collect.visit_program(program);
    let is_real = |span: Span| span.start < span.end && span.end <= limit;
    let spans = settled(&collect.entries, is_real);
    staggered(spans, &collect.entries, program.source_text)
}

fn inside(text: &str, span: Span, chars: usize) -> Span {
    let end = (span.end as usize).min(text.len());
    let rest = text.get(span.start as usize..end).unwrap_or_default();
    let skipped = rest.char_indices().nth(chars).map(|(at, _)| at);
    match skipped.and_then(|at| u32::try_from(at).ok()) {
        Some(at) => Span::new(span.start + at, span.end),
        None => span,
    }
}

fn staggered(spans: Vec<Span>, entries: &[Entry], text: &str) -> Vec<Span> {
    let mut depth = vec![0usize; spans.len()];
    for index in (1..spans.len().saturating_sub(1)).rev() {
        let nested = entries[index + 1].parent as usize == index;
        if nested && !spans[index].is_empty() && spans[index + 1].start == spans[index].start {
            depth[index] = if depth[index + 1] == 1 { 2 } else { 1 };
        }
    }
    spans
        .into_iter()
        .zip(depth)
        .map(|(span, depth)| inside(text, span, depth))
        .collect()
}

fn assign(program: &mut Program<'_>, spans: Vec<Span>) {
    let mut assign = Assign {
        spans: spans.into_iter(),
    };
    assign.visit_program(program);
}

fn window_of(spans: &[Span], text: &str) -> (usize, usize) {
    let mut real = spans.iter().filter(|span| !span.is_empty());
    let Some(first) = real.next() else {
        return (0, 0);
    };
    let (start, end) = real.fold((first.start, first.end), |(start, end), span| {
        (start.min(span.start), end.max(span.end))
    });
    let (mut start, mut end) = (start as usize, (end as usize).min(text.len()));
    while !text.is_char_boundary(start) {
        start -= 1;
    }
    while !text.is_char_boundary(end) {
        end += 1;
    }
    (start, end)
}

pub fn fill_spans(program: &mut Program<'_>, origins: &Origins) {
    let whole = program.span;
    let spans = filled(program, origins);
    assign(program, spans);
    program.span = whole;
}

pub fn fill_window(program: &mut Program<'_>, origins: &Origins) -> usize {
    let spans = filled(program, origins);
    let text = program.source_text;
    let (start, end) = window_of(&spans, text);
    let shift = u32::try_from(start).unwrap_or_default();
    let moved = spans
        .into_iter()
        .map(|span| match span.is_empty() {
            true => span,
            false => Span::new(span.start - shift, span.end - shift),
        })
        .collect();
    assign(program, moved);
    program.source_text = &text[start..end];
    program.span = Span::new(0, u32::try_from(end - start).unwrap_or_default());
    start
}
