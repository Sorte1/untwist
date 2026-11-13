use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use rustc_hash::FxHashSet;

use crate::rollup::scan::Facts;

const PRIVATE_REACH: usize = 3;
const SIMILAR: f64 = 1.0 / 3.0;
const DECONFLICT_REACH: usize = 40;
const DECONFLICT: char = '$';
const MIN_STEM: usize = 3;
const FREEZE: &str = "freeze";
const OBJECT: &str = "Object";
const PROTO: &str = "__proto__";
const MODULE_STEM: &str = "module";

pub use super::plan::Run;

pub struct Input<'p> {
    pub entry: Vec<u32>,
    pub fixed: &'p [(u32, u32)],
    pub refs: &'p [Vec<u32>],
    pub held: &'p [bool],
}

struct Line<'p, 'a> {
    scoping: &'p Scoping,
    items: &'p [Statement<'a>],
    facts: &'p Facts,
    entry: &'p [u32],
    fixed: &'p [(u32, u32)],
    position: Vec<Option<usize>>,
    refs: Vec<Vec<usize>>,
}

impl Line<'_, '_> {
    fn len(&self) -> usize {
        self.entry.len()
    }

    fn statement(&self, at: usize) -> &Statement<'_> {
        &self.items[self.entry[at] as usize]
    }

    fn owner_position(&self, symbol: oxc_syntax::symbol::SymbolId) -> Option<usize> {
        let owner = *self.facts.owner.get(symbol.index())?;
        self.position.get(owner as usize).copied().flatten()
    }

    fn reference_owner(
        &self,
        reference: &IdentifierReference<'_>,
    ) -> Option<Result<usize, String>> {
        let id = reference.reference_id.get()?;
        match self.scoping.get_reference(id).symbol_id() {
            Some(symbol) => self.owner_position(symbol).map(Ok),
            None => Some(Err(reference.name.to_string())),
        }
    }
}

struct Glue {
    depth: Vec<i64>,
}

impl Glue {
    fn new(n: usize) -> Self {
        Self {
            depth: vec![0; n + 2],
        }
    }

    fn join(&mut self, a: usize, b: usize) {
        let (low, high) = (a.min(b), a.max(b));
        if low == high {
            return;
        }
        self.depth[low + 1] += 1;
        self.depth[high + 1] -= 1;
    }

    fn glued(&self, n: usize) -> Vec<bool> {
        let mut out = vec![false; n];
        let mut running = 0;
        for (at, slot) in out.iter_mut().enumerate() {
            running += self.depth[at];
            *slot = at > 0 && running > 0;
        }
        out
    }
}

fn atoms(glued: &[bool]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (at, &joined) in glued.iter().enumerate() {
        match out.last_mut() {
            Some(last) if joined => last.1 = at,
            _ => out.push((at, at)),
        }
    }
    out
}

fn namespace_members(line: &Line<'_, '_>, at: usize) -> Option<Vec<usize>> {
    let Statement::VariableDeclaration(declaration) = line.statement(at) else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let Expression::CallExpression(call) = declarator.init.as_ref()?.without_parentheses() else {
        return None;
    };
    let Expression::StaticMemberExpression(callee) = &call.callee else {
        return None;
    };
    let Expression::Identifier(object) = &callee.object else {
        return None;
    };
    let global = matches!(line.reference_owner(object), Some(Err(_)));
    if callee.property.name != FREEZE || object.name != OBJECT || !global {
        return None;
    }
    let [Argument::ObjectExpression(members)] = call.arguments.as_slice() else {
        return None;
    };
    let mut proto = false;
    let mut found = Vec::new();
    for property in &members.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        if property.key.static_name().is_some_and(|name| name == PROTO) {
            proto |= matches!(property.value, Expression::NullLiteral(_));
            continue;
        }
        found.push(member_owner(line, &property.value)?);
    }
    (proto && !found.is_empty()).then_some(found)
}

fn member_owner(line: &Line<'_, '_>, value: &Expression<'_>) -> Option<usize> {
    let reference = match value.without_parentheses() {
        Expression::Identifier(reference) => reference,
        Expression::FunctionExpression(function) => {
            let body = function.body.as_ref()?;
            let [Statement::ReturnStatement(returned)] = body.statements.as_slice() else {
                return None;
            };
            let Some(Expression::Identifier(reference)) = &returned.argument else {
                return None;
            };
            reference
        }
        _ => return None,
    };
    line.reference_owner(reference)?.ok()
}

fn plain_data(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        Expression::UnaryExpression(unary) => plain_data(&unary.argument),
        _ => false,
    }
}

fn identifier_shape(line: &Line<'_, '_>, expression: &Expression<'_>) -> Option<String> {
    let Expression::Identifier(reference) = expression.without_parentheses() else {
        return None;
    };
    Some(match line.reference_owner(reference)? {
        Ok(at) => format!("item{at}"),
        Err(name) => format!("global {name}"),
    })
}

fn shape(line: &Line<'_, '_>, at: usize) -> Option<String> {
    let leaf = line.refs[at].is_empty();
    match line.statement(at) {
        Statement::FunctionDeclaration(_) => leaf.then(|| "function".to_string()),
        Statement::ClassDeclaration(class) => {
            let base = identifier_shape(line, &class.heritage.as_ref()?.expression)?;
            Some(format!("class extends {base}"))
        }
        Statement::VariableDeclaration(declaration) => {
            let [declarator] = declaration.declarations.as_slice() else {
                return None;
            };
            let Some(init) = &declarator.init else {
                return Some("data".to_string());
            };
            match init.without_parentheses() {
                Expression::ObjectExpression(_) | Expression::ArrayExpression(_) => {
                    leaf.then(|| "data".to_string())
                }
                Expression::NewExpression(new) => {
                    let base = identifier_shape(line, &new.callee)?;
                    Some(format!("new {base}"))
                }
                other => plain_data(other).then(|| "data".to_string()),
            }
        }
        _ => None,
    }
}

fn base_glue(line: &Line<'_, '_>, glue: &mut Glue) -> Vec<usize> {
    for (at, targets) in line.refs.iter().enumerate() {
        for &target in targets {
            if target > at {
                glue.join(at, target);
            }
        }
    }
    let pairs = line.fixed.iter().chain(&line.facts.spans);
    for &(a, b) in pairs {
        if let (Some(a), Some(b)) = (line.position[a as usize], line.position[b as usize]) {
            glue.join(a, b);
        }
    }
    let mut forced = Vec::new();
    for at in 0..line.len() {
        if let Some(members) = namespace_members(line, at) {
            if let Some(&first) = members.iter().min() {
                glue.join(first.min(at), at);
            }
            forced.push(at + 1);
        }
    }
    forced
}

fn users(line: &Line<'_, '_>) -> Vec<Vec<usize>> {
    let mut by = vec![Vec::new(); line.len()];
    for (at, targets) in line.refs.iter().enumerate() {
        for &target in targets {
            by[target].push(at);
        }
    }
    by
}

fn private_glue(line: &Line<'_, '_>, glue: &mut Glue, pieces: &[(usize, usize)]) {
    let by = users(line);
    let mut piece_of = vec![0; line.len()];
    for (index, &(start, end)) in pieces.iter().enumerate() {
        piece_of[start..=end].fill(index);
    }
    for (index, &(start, end)) in pieces.iter().enumerate() {
        let consumers: Vec<usize> = (start..=end)
            .flat_map(|at| by[at].iter().copied())
            .filter(|&user| piece_of[user] != index)
            .collect();
        let Some(far) = consumers.iter().map(|&user| piece_of[user]).max() else {
            continue;
        };
        let later = consumers.iter().all(|&user| piece_of[user] > index);
        if !later || far - index > PRIVATE_REACH {
            continue;
        }
        let region_end = pieces[far].1;
        let consumed: FxHashSet<usize> = consumers
            .iter()
            .flat_map(|&user| line.refs[user].iter().copied())
            .collect();
        let covered = (start..=end).all(|at| {
            line.refs[at]
                .iter()
                .all(|&target| (start..=region_end).contains(&target) || consumed.contains(&target))
        });
        if covered {
            glue.join(start, pieces[far].0);
        }
    }
}

fn declares_nothing(line: &Line<'_, '_>, (start, end): (usize, usize)) -> bool {
    (start..=end).all(|at| line.facts.declares[line.entry[at] as usize] == 0)
}

fn neighbour_glue(
    line: &Line<'_, '_>,
    glue: &mut Glue,
    pieces: &[(usize, usize)],
    forced: &[usize],
) {
    for pair in pieces.windows(2) {
        let (before, after) = (pair[0], pair[1]);
        if forced.contains(&after.0) {
            continue;
        }
        if declares_nothing(line, after) {
            glue.join(before.1, after.0);
            continue;
        }
        let single = before.0 == before.1 && after.0 == after.1;
        if single && shape(line, before.0).is_some_and(|known| Some(known) == shape(line, after.0))
        {
            glue.join(before.1, after.0);
        }
    }
}

fn similarity(a: &FxHashSet<usize>, b: &FxHashSet<usize>) -> Option<f64> {
    let shared = a.iter().filter(|x| b.contains(x)).count();
    let union = a.len() + b.len() - shared;
    (union > 0).then(|| shared as f64 / union as f64)
}

fn deconflict_base(name: &str) -> &str {
    match name.rsplit_once(DECONFLICT) {
        Some((base, suffix))
            if !base.is_empty()
                && !suffix.is_empty()
                && suffix.bytes().all(|b| b.is_ascii_digit()) =>
        {
            base
        }
        _ => name,
    }
}

fn declared_names(line: &Line<'_, '_>) -> Vec<Vec<String>> {
    let mut names = vec![Vec::new(); line.len()];
    for (symbol, &owner) in line.facts.owner.iter().enumerate() {
        let Some(at) = line.position.get(owner as usize).copied().flatten() else {
            continue;
        };
        let symbol = oxc_syntax::symbol::SymbolId::from_usize(symbol);
        names[at].push(deconflict_base(line.scoping.symbol_name(symbol)).to_string());
    }
    names
}

struct Signals<'l, 'p, 'a> {
    line: &'l Line<'p, 'a>,
    by: Vec<Vec<usize>>,
    piece_of: Vec<usize>,
    names: Vec<Vec<String>>,
}

impl Signals<'_, '_, '_> {
    fn consumers(&self, (start, last): (usize, usize), beyond: usize) -> FxHashSet<usize> {
        (start..=last)
            .flat_map(|at| self.by[at].iter().copied())
            .filter(|&user| user > beyond)
            .map(|user| self.piece_of[user])
            .collect()
    }

    fn imports(&self, (start, last): (usize, usize), before: usize) -> FxHashSet<usize> {
        (start..=last)
            .flat_map(|at| self.line.refs[at].iter().copied())
            .filter(|&target| target < before)
            .collect()
    }

    fn redeclares(&self, before: (usize, usize), after: (usize, usize)) -> bool {
        let earlier: FxHashSet<&str> = (before.0.saturating_sub(DECONFLICT_REACH)..=before.1)
            .flat_map(|at| self.names[at].iter().map(String::as_str))
            .collect();
        (after.0..=after.1)
            .flat_map(|at| self.names[at].iter())
            .any(|name| earlier.contains(name.as_str()))
    }

    fn separates(&self, before: (usize, usize), after: (usize, usize)) -> bool {
        if self.redeclares(before, after) {
            return true;
        }
        let consumed_a = self.consumers(before, after.1);
        let consumed_b = self.consumers(after, after.1);
        let used_later = !consumed_a.is_empty();
        let consumers = similarity(&consumed_a, &consumed_b).is_none_or(|value| value <= SIMILAR);
        let imports = similarity(
            &self.imports(before, before.0),
            &self.imports(after, before.0),
        )
        .is_none_or(|value| value <= SIMILAR);
        let uses_before = (after.0..=after.1).any(|at| {
            self.line.refs[at]
                .iter()
                .any(|&target| (before.0..=before.1).contains(&target))
        });
        (used_later || !uses_before) && consumers && imports
    }
}

fn strong(line: &Line<'_, '_>, pieces: &[(usize, usize)], forced: &[usize]) -> Vec<(usize, usize)> {
    let mut piece_of = vec![0; line.len()];
    for (index, &(start, last)) in pieces.iter().enumerate() {
        piece_of[start..=last].fill(index);
    }
    let signals = Signals {
        line,
        by: users(line),
        piece_of,
        names: declared_names(line),
    };
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (index, &piece) in pieces.iter().enumerate() {
        let cut =
            index == 0 || forced.contains(&piece.0) || signals.separates(pieces[index - 1], piece);
        match out.last_mut() {
            Some(last) if !cut => last.1 = piece.1,
            _ => out.push(piece),
        }
    }
    out
}

fn piece_start(pieces: &[(usize, usize)], at: usize) -> usize {
    let index = pieces.partition_point(|&(_, last)| last < at);
    pieces.get(index).map_or(at, |&(start, _)| start)
}

fn final_start(line: &Line<'_, '_>, pieces: &[(usize, usize)], held: &[bool]) -> usize {
    let n = line.len();
    let first = (0..n)
        .find(|&at| held[line.entry[at] as usize])
        .unwrap_or_else(|| pieces.last().map_or(0, |&(start, _)| start));
    let mut end = piece_start(pieces, first);
    let mut earliest_user = vec![usize::MAX; n + 1];
    for (at, targets) in line.refs.iter().enumerate() {
        for &target in targets {
            if target > at {
                earliest_user[target] = earliest_user[target].min(at);
            }
        }
    }
    let mut reach = vec![usize::MAX; n + 1];
    for at in (0..n).rev() {
        reach[at] = reach[at + 1].min(earliest_user[at]);
    }
    while end > 0 && reach[end] < end {
        end = piece_start(pieces, reach[end]);
    }
    end
}

fn stem(
    scoping: &Scoping,
    line: &Line<'_, '_>,
    members: &[usize],
    used: &FxHashSet<usize>,
    n: usize,
) -> String {
    let named = members
        .iter()
        .filter(|at| used.contains(at))
        .find_map(|&at| {
            let id = match line.statement(at) {
                Statement::ClassDeclaration(class) => class.id.as_ref()?,
                Statement::FunctionDeclaration(function) => function.id.as_ref()?,
                _ => return None,
            };
            let symbol = id.symbol_id.get()?;
            let name = scoping.symbol_name(symbol);
            let plain = name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$');
            (plain && name.len() >= MIN_STEM).then(|| name.to_string())
        });
    named.unwrap_or_else(|| format!("{MODULE_STEM}{n}"))
}

pub fn runs(
    scoping: &Scoping,
    items: &[Statement<'_>],
    facts: &Facts,
    input: &Input<'_>,
) -> Vec<Run> {
    let mut position = vec![None; items.len()];
    for (at, &item) in input.entry.iter().enumerate() {
        position[item as usize] = Some(at);
    }
    let refs: Vec<Vec<usize>> = input
        .entry
        .iter()
        .map(|&item| {
            let mut found: Vec<usize> = input.refs[item as usize]
                .iter()
                .filter_map(|&target| position[target as usize])
                .collect();
            found.sort_unstable();
            found.dedup();
            found
        })
        .collect();
    let line = Line {
        scoping,
        items,
        facts,
        entry: &input.entry,
        fixed: input.fixed,
        position,
        refs,
    };
    let n = line.len();
    if n < 2 {
        return Vec::new();
    }
    let mut glue = Glue::new(n);
    let forced = base_glue(&line, &mut glue);
    let first = atoms(&glue.glued(n));
    private_glue(&line, &mut glue, &first);
    let second = atoms(&glue.glued(n));
    neighbour_glue(&line, &mut glue, &second, &forced);
    let pieces = strong(&line, &atoms(&glue.glued(n)), &forced);
    let end = final_start(&line, &pieces, input.held);
    let kept: Vec<(usize, usize)> = pieces.into_iter().filter(|&(_, last)| last < end).collect();
    let mut piece_of = vec![usize::MAX; n];
    for (index, &(start, last)) in kept.iter().enumerate() {
        piece_of[start..=last].fill(index);
    }
    let used: FxHashSet<usize> = line
        .refs
        .iter()
        .enumerate()
        .flat_map(|(at, targets)| targets.iter().map(move |&target| (at, target)))
        .filter(|&(at, target)| piece_of[at] != piece_of[target])
        .map(|(_, target)| target)
        .collect();
    kept.iter()
        .enumerate()
        .map(|(index, &(start, last))| {
            let members: Vec<usize> = (start..=last).collect();
            Run {
                stem: stem(scoping, &line, &members, &used, index + 1),
                members: members.iter().map(|&at| line.entry[at]).collect(),
            }
        })
        .collect()
}
