use super::super::guards::{holds_a_line_terminator, Guard};
use super::walk::{Target, MARKER};

fn offset(at: usize) -> Option<u32> {
    u32::try_from(at).ok()
}

fn indented(printed: &str, indent: usize) -> String {
    let mut shifted = String::with_capacity(printed.len());
    for (index, line) in printed.split('\n').enumerate() {
        if index != 0 {
            shifted.push('\n');
            shifted.extend(std::iter::repeat_n(' ', indent));
        }
        shifted.push_str(line);
    }
    shifted
}

fn locate(code: &str, marker: usize, target: Target) -> Option<Guard> {
    let start = marker.checked_sub(target.prefix.len())?;
    (code.get(start..marker)? == target.prefix).then_some(())?;
    let line = code.get(..start)?.rfind('\n').map_or(0, |at| at + 1);
    let indent = code
        .get(line..start)?
        .bytes()
        .take_while(|byte| *byte == b' ')
        .count();
    let expected = indented(&target.printed, indent);
    let end = start.checked_add(expected.len())?;
    let printed = code.get(start..end)?;
    (printed == expected && holds_a_line_terminator(printed)).then_some(())?;
    Some((offset(start)?, offset(end)?, target.text))
}

pub(super) fn located(code: &str, targets: Vec<Target>) -> Option<Vec<Guard>> {
    if targets.is_empty() {
        return Some(Vec::new());
    }
    let mut markers = code.match_indices(MARKER).map(|(at, _)| at);
    let mut guards = Vec::with_capacity(targets.len());
    for target in targets {
        guards.push(locate(code, markers.next()?, target)?);
    }
    markers.next().is_none().then_some(guards)
}
