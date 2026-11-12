use std::ops::Range;

use super::scan::Facts;
use super::wrappers::Group;

const WINDOW: usize = 3;
const MIN_DECLARATIONS: u32 = 2;

struct Graph {
    refs: Vec<Vec<u32>>,
    by: Vec<Vec<u32>>,
}

fn graph(facts: &Facts) -> Graph {
    let n = facts.declares.len();
    let mut refs = vec![Vec::new(); n];
    for &(item, symbol) in &facts.uses {
        refs[item as usize].push(facts.owner[symbol.index()]);
    }
    let mut by = vec![Vec::new(); n];
    for (item, targets) in refs.iter_mut().enumerate() {
        targets.sort_unstable();
        targets.dedup();
        for &target in targets.iter() {
            by[target as usize].push(item as u32);
        }
    }
    Graph { refs, by }
}

struct Union {
    parent: Vec<u32>,
    members: Vec<Vec<u32>>,
    group: Vec<Option<u32>>,
}

impl Union {
    fn new(n: usize, groups: &[Group]) -> Self {
        let mut group = vec![None; n];
        for (index, found) in groups.iter().enumerate() {
            for &member in &found.members {
                group[member as usize] = Some(index as u32);
            }
        }
        Self {
            parent: (0..n as u32).collect(),
            members: (0..n as u32).map(|i| vec![i]).collect(),
            group,
        }
    }

    fn join(&mut self, a: u32, b: u32) {
        let (left, right) = (self.group[a as usize], self.group[b as usize]);
        if left == right || (left.is_none() && right.is_none()) {
            self.union(a, b);
        }
    }

    fn join_groups(&mut self, groups: &[Group]) {
        for found in groups {
            for pair in found.members.windows(2) {
                self.union(pair[0], pair[1]);
            }
        }
    }

    fn find(&mut self, mut x: u32) -> u32 {
        while self.parent[x as usize] != x {
            let up = self.parent[self.parent[x as usize] as usize];
            self.parent[x as usize] = up;
            x = up;
        }
        x
    }

    fn union(&mut self, a: u32, b: u32) {
        let (a, b) = (self.find(a), self.find(b));
        if a == b {
            return;
        }
        let (big, small) = match self.members[a as usize].len() >= self.members[b as usize].len() {
            true => (a, b),
            false => (b, a),
        };
        self.parent[small as usize] = big;
        let moved = std::mem::take(&mut self.members[small as usize]);
        self.members[big as usize].extend(moved);
    }
}

fn shares(a: &[u32], b: &[u32]) -> bool {
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => return true,
        }
    }
    false
}

fn mutual(g: &Graph, uf: &mut Union) {
    for (i, targets) in g.refs.iter().enumerate() {
        for &j in targets {
            if g.refs[j as usize].binary_search(&(i as u32)).is_ok() {
                uf.join(i as u32, j);
            }
        }
    }
}

fn chain(g: &Graph, uf: &mut Union) {
    for i in 1..g.refs.len() {
        let prev = (i - 1) as u32;
        if g.refs[i].binary_search(&prev).is_err() {
            continue;
        }
        let root = uf.find(prev);
        if g.refs[i].iter().all(|&t| uf.find(t) == root) {
            uf.join(prev, i as u32);
        }
    }
}

fn inert_helpers(g: &Graph, facts: &Facts, uf: &mut Union) {
    for b in 0..g.refs.len() {
        if facts.declares[b] == 0 || !g.refs[b].is_empty() {
            continue;
        }
        if let [consumer] = g.by[b][..] {
            if (consumer as usize).abs_diff(b) <= WINDOW {
                uf.join(b as u32, consumer);
            }
        }
    }
}

fn adjacent_shared(g: &Graph, facts: &Facts, uf: &mut Union) {
    let n = g.refs.len();
    for i in 0..n {
        if facts.declares[i] == 0 {
            continue;
        }
        for j in (i + 1)..(i + WINDOW + 1).min(n) {
            if facts.declares[j] == 0 || uf.find(i as u32) == uf.find(j as u32) {
                continue;
            }
            if shares(&g.refs[i], &g.refs[j]) {
                uf.join(i as u32, j as u32);
            }
        }
    }
}

fn narrow_cluster(g: &Graph, uf: &mut Union, root: u32, other: u32) -> bool {
    let mut seen = None;
    let members = std::mem::take(&mut uf.members[root as usize]);
    let mut narrow = true;
    'outer: for &member in &members {
        for &target in &g.refs[member as usize] {
            let at = uf.find(target);
            if at == root || at == other || seen == Some(at) {
                continue;
            }
            if seen.is_some() {
                narrow = false;
                break 'outer;
            }
            seen = Some(at);
        }
    }
    uf.members[root as usize] = members;
    narrow
}

fn exclusive_consumers(g: &Graph, facts: &Facts, uf: &mut Union) {
    for b in 0..g.refs.len() {
        if facts.declares[b] == 0 {
            continue;
        }
        let [consumer] = g.by[b][..] else {
            continue;
        };
        let (root, other) = (uf.find(consumer), uf.find(b as u32));
        if root == other || (consumer as usize).abs_diff(b) > WINDOW {
            continue;
        }
        if narrow_cluster(g, uf, root, other) {
            uf.join(b as u32, consumer);
        }
    }
}

fn writer_owner(facts: &Facts, uf: &mut Union, spans: &mut Vec<(u32, u32)>) {
    for &(writer, owner) in &facts.writes {
        uf.union(writer, owner);
        spans.push((writer.min(owner), writer.max(owner)));
    }
}

fn constraints(facts: &Facts, g: &Graph, uf: &mut Union) -> Vec<(u32, u32)> {
    let n = g.refs.len() as u32;
    let mut spans = facts.spans.clone();
    for &(a, b) in &facts.spans {
        uf.union(a, b);
    }
    writer_owner(facts, uf, &mut spans);
    for (item, targets) in g.refs.iter().enumerate() {
        if let Some(&last) = targets.last() {
            if last > item as u32 {
                spans.push((item as u32, last));
            }
        }
    }
    for (item, &pinned) in facts.pinned.iter().enumerate() {
        if pinned {
            spans.push((item as u32, n - 1));
        }
    }
    for root in 0..n {
        let members = &uf.members[root as usize];
        let declared: u32 = members
            .iter()
            .map(|&m| facts.declares[m as usize].min(1))
            .sum();
        if uf.parent[root as usize] == root && declared >= MIN_DECLARATIONS {
            let low = members.iter().copied().min().unwrap_or(root);
            let high = members.iter().copied().max().unwrap_or(root);
            spans.push((low, high));
        }
    }
    spans
}

fn covered(mut spans: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    spans.sort_unstable();
    let mut merged: Vec<(u32, u32)> = Vec::new();
    for (low, high) in spans {
        match merged.last_mut() {
            Some(last) if low <= last.1 => last.1 = last.1.max(high),
            _ => merged.push((low, high)),
        }
    }
    merged
}

fn gap_split(declares: &[u32], from: usize, to: usize) -> usize {
    declares[from..to]
        .iter()
        .rposition(|&d| d > 0)
        .map_or(from, |at| from + at + 1)
}

fn segments(merged: &[(u32, u32)], declares: &[u32]) -> Vec<Range<usize>> {
    let n = declares.len();
    let mut out: Vec<Range<usize>> = Vec::new();
    let mut start = 0;
    for (index, &(_, high)) in merged.iter().enumerate() {
        let end = match merged.get(index + 1) {
            Some(next) => gap_split(declares, high as usize + 1, next.0 as usize),
            None => high as usize + 1,
        };
        out.push(start..end);
        start = end;
    }
    if start < n {
        out.push(start..n);
    }
    out
}

fn fold_small(segments: Vec<Range<usize>>, facts: &Facts) -> Vec<Range<usize>> {
    let declared = |range: &Range<usize>| -> u32 {
        facts.declares[range.clone()]
            .iter()
            .map(|&d| d.min(1))
            .sum()
    };
    let count = segments.len();
    let mut out: Vec<Range<usize>> = Vec::new();
    let mut pending: Option<usize> = None;
    for (index, segment) in segments.into_iter().enumerate() {
        let start = pending.take().unwrap_or(segment.start);
        let segment = start..segment.end;
        if index + 1 == count || declared(&segment) >= MIN_DECLARATIONS {
            out.push(segment);
        } else if let Some(last) = out.last_mut() {
            last.end = segment.end;
        } else {
            pending = Some(segment.start);
        }
    }
    out
}

pub fn plan(facts: &Facts, groups: &[Group]) -> Option<Vec<Range<usize>>> {
    let g = graph(facts);
    let n = g.refs.len();
    let mut uf = Union::new(n, groups);
    uf.join_groups(groups);
    mutual(&g, &mut uf);
    chain(&g, &mut uf);
    inert_helpers(&g, facts, &mut uf);
    adjacent_shared(&g, facts, &mut uf);
    exclusive_consumers(&g, facts, &mut uf);
    let spans = constraints(facts, &g, &mut uf);
    let planned = fold_small(segments(&covered(spans), &facts.declares), facts);
    (planned.len() >= 2).then_some(planned)
}
