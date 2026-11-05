use super::super::Elements;
use super::walk::Shape;

const STEP: usize = 2;

pub(super) struct Bracketed {
    open: u32,
    close: u32,
    indent: usize,
    starts: Vec<u32>,
}

struct Frame {
    index: usize,
    open: u32,
    indent: Option<usize>,
    starts: Vec<u32>,
}

pub(super) struct Site {
    bounds: (u32, u32),
    elements: Vec<(u32, u32)>,
}

impl Elements for Site {
    fn bounds(&self) -> (u32, u32) {
        self.bounds
    }

    fn count(&self) -> usize {
        self.elements.len()
    }

    fn elided(&self) -> bool {
        false
    }

    fn element(&self, index: usize) -> (u32, u32) {
        self.elements[index]
    }
}

struct Lines {
    opened: Vec<Option<Bracketed>>,
    stack: Vec<Frame>,
}

impl Lines {
    fn close(&mut self, at: u32, indent: usize) -> Option<()> {
        let frame = self.stack.pop()?;
        if frame.indent != Some(indent + STEP) {
            return None;
        }
        let slot = self.opened.get_mut(frame.index)?;
        *slot = Some(Bracketed {
            open: frame.open,
            close: at,
            indent: indent + STEP,
            starts: frame.starts,
        });
        Some(())
    }

    fn element(&mut self, at: u32, indent: usize, first: u8) {
        let Some(top) = self.stack.last_mut() else {
            return;
        };
        let expected = *top.indent.get_or_insert(indent);
        if indent == expected && !matches!(first, b'}' | b',') {
            top.starts.push(at);
        }
    }

    fn open(&mut self, at: u32) {
        self.stack.push(Frame {
            index: self.opened.len(),
            open: at,
            indent: None,
            starts: Vec::new(),
        });
        self.opened.push(None);
    }

    fn line(&mut self, line: &[u8], start: u32) -> Option<()> {
        let indent = line.iter().take_while(|byte| **byte == b' ').count();
        let at = start + u32::try_from(indent).ok()?;
        match line.get(indent) {
            Some(b']') => self.close(at, indent)?,
            Some(first) => self.element(at, indent, *first),
            None => {}
        }
        if line.last() == Some(&b'[') {
            let end = start + u32::try_from(line.len()).ok()?;
            self.open(end - 1);
        }
        Some(())
    }
}

pub(super) fn bracketed(code: &str, expected: usize) -> Option<Vec<Bracketed>> {
    let mut lines = Lines {
        opened: Vec::with_capacity(expected),
        stack: Vec::new(),
    };
    let mut start = 0;
    for line in code.split('\n') {
        let line = line.as_bytes();
        lines.line(line, u32::try_from(start).ok()?)?;
        start += line.len() + 1;
        if lines.opened.len() > expected {
            return None;
        }
    }
    if !lines.stack.is_empty() || lines.opened.len() != expected {
        return None;
    }
    lines.opened.into_iter().collect()
}

fn ends(bytes: &[u8], found: &Bracketed) -> Option<Vec<u32>> {
    let step = u32::try_from(found.indent).ok()?;
    let mut ends = Vec::with_capacity(found.starts.len());
    for next in found.starts.iter().skip(1) {
        let end = next.checked_sub(step + 2)?;
        let separated = bytes.get(end as usize..end as usize + 2) == Some(b",\n");
        separated.then_some(())?;
        ends.push(end);
    }
    let last = found.close.checked_sub(step - 1)?;
    (bytes.get(last as usize) == Some(&b'\n')).then_some(())?;
    ends.push(last);
    Some(ends)
}

pub(super) enum Placed {
    Site(Site),
    Skipped,
}

fn region(bytes: &[u8], (from, to): (u32, u32)) -> Option<&[u8]> {
    let text = bytes.get(from as usize..to as usize)?;
    (!text.is_empty() && text.last() != Some(&b' ')).then_some(text)
}

fn opaque(text: &[u8], object: bool) -> bool {
    text.starts_with(b"/*") || text.starts_with(b"//") || (object && text.starts_with(b"("))
}

pub(super) fn site(bytes: &[u8], shape: &Shape, found: Bracketed) -> Option<Placed> {
    if shape.elided {
        return Some(Placed::Skipped);
    }
    if found.starts.len() != shape.count {
        return None;
    }
    let ends = ends(bytes, &found)?;
    let elements: Vec<(u32, u32)> = found.starts.iter().copied().zip(ends).collect();
    let mut skipped = shape.wrapped;
    for (index, element) in elements.iter().enumerate() {
        let object = shape.objects.contains(&index);
        skipped |= opaque(region(bytes, *element)?, object);
    }
    if skipped {
        return Some(Placed::Skipped);
    }
    Some(Placed::Site(Site {
        bounds: (found.open, found.close + 1),
        elements,
    }))
}
