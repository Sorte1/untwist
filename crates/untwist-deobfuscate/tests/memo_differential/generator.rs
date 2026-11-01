use std::fmt::Write;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }

    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }

    fn between(&mut self, low: i64, high: i64) -> i64 {
        low + (self.next() % (high - low + 1) as u64) as i64
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    Base,
    Ternary,
    SelfMember,
    GateAbsent,
    GateUnfired,
    Extra,
    Wrapped,
    OutOfRange,
    CallMutant,
    MemberMutant,
    WriteMutant,
    FunctionMutant,
    NestedMutant,
    MappedParams,
    UserPrototype,
}

pub const CLASSES: [Class; 15] = [
    Class::Base,
    Class::Ternary,
    Class::SelfMember,
    Class::GateAbsent,
    Class::GateUnfired,
    Class::Extra,
    Class::Wrapped,
    Class::OutOfRange,
    Class::CallMutant,
    Class::MemberMutant,
    Class::WriteMutant,
    Class::FunctionMutant,
    Class::NestedMutant,
    Class::MappedParams,
    Class::UserPrototype,
];

impl Class {
    pub fn mutant(self) -> bool {
        matches!(
            self,
            Class::CallMutant
                | Class::MemberMutant
                | Class::WriteMutant
                | Class::FunctionMutant
                | Class::NestedMutant
        )
    }
}

struct Array {
    items: Vec<Option<i64>>,
    rotation: usize,
}

impl Array {
    fn generate(rng: &mut Rng) -> Self {
        let size = 6 + rng.below(11);
        let items = (0..size)
            .map(|_| (!rng.chance(30)).then(|| rng.between(1, 999)))
            .collect();
        let rotation = 1 + rng.below(size - 1);
        Self { items, rotation }
    }

    fn literal(&self) -> String {
        let texts: Vec<String> = self
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| match item {
                Some(number) => format!("'{number}q{index}'"),
                None => format!("'z{index}'"),
            })
            .collect();
        texts.join(", ")
    }

    fn text(&self, index: usize) -> String {
        match self.items[index] {
            Some(number) => format!("{number}q{index}"),
            None => format!("z{index}"),
        }
    }

    fn prototype_keys(&self) -> String {
        let first = self.text(0);
        let keys: Vec<String> = (0..self.items.len())
            .map(|index| format!("'{index}{first}': 'w!'"))
            .collect();
        keys.join(", ")
    }

    fn at(&self, index: usize, attempt: usize) -> Option<i64> {
        self.items[(index + attempt) % self.items.len()]
    }

    fn live(&self, rng: &mut Rng) -> usize {
        loop {
            let index = rng.below(self.items.len());
            if self.at(index, self.rotation).is_some() {
                return index;
            }
        }
    }

    fn opening(&self) -> Option<usize> {
        (0..self.items.len())
            .find(|index| self.at(*index, 0).is_none() && self.at(*index, self.rotation).is_some())
    }
}

fn number(rng: &mut Rng, value: i64) -> String {
    let text = if rng.chance(50) {
        format!("0x{:x}", value.unsigned_abs())
    } else {
        value.unsigned_abs().to_string()
    };
    if value < 0 {
        format!("-{text}")
    } else {
        text
    }
}

fn offset_expression(rng: &mut Rng, value: i64) -> String {
    let first = rng.between(-0x2000, 0x2000);
    let second = rng.between(-0x200, 0x200);
    let third = value - first - second;
    let mut text = number(rng, first);
    for part in [second, third] {
        let rendered = number(rng, part.abs());
        let sign = if part < 0 { '-' } else { '+' };
        let _ = write!(text, "{sign}{rendered}");
    }
    text
}

struct Decoder {
    name: &'static str,
    flag: &'static str,
    offset: i64,
    shift_up: bool,
}

impl Decoder {
    fn argument(&self, index: usize) -> i64 {
        let index = index as i64;
        if self.shift_up {
            index - self.offset
        } else {
            index + self.offset
        }
    }
}

struct Shape {
    class: Class,
    ternary: bool,
    self_member: bool,
    gate: bool,
    out_of_range: bool,
    prototype: Option<String>,
}

fn shape(rng: &mut Rng, class: Class) -> Shape {
    Shape {
        class,
        ternary: class == Class::Ternary
            || (!matches!(class, Class::Base | Class::MappedParams) && rng.chance(30)),
        self_member: matches!(class, Class::SelfMember | Class::UserPrototype)
            || (class != Class::Base && class != Class::MappedParams && rng.chance(25)),
        gate: class != Class::GateAbsent,
        out_of_range: class == Class::OutOfRange,
        prototype: None,
    }
}

fn container(shape: &Shape, decoder: &Decoder) -> String {
    match (shape.gate, shape.self_member) {
        (false, _) => "m".to_string(),
        (true, true) => format!("{}['m']", decoder.name),
        (true, false) => "c".to_string(),
    }
}

fn gate(rng: &mut Rng, shape: &Shape, decoder: &Decoder) -> String {
    let name = decoder.name;
    let flag = decoder.flag;
    let nested = match shape.class {
        Class::NestedMutant => [
            "try { } catch (j) { } ",
            "{ let n = 0; } ",
            "for (let f = 0; f < 1; f++) { } ",
        ][rng.below(3)],
        _ => "",
    };
    let store = match &shape.prototype {
        Some(prototype) => format!("Object.create({{ {prototype} }})"),
        None => "{}".to_string(),
    };
    match (shape.gate, shape.self_member) {
        (false, _) => String::new(),
        (true, true) => {
            format!("    if ({name}['{flag}'] === undefined) {{ {nested}{name}['m'] = {store}; {name}['{flag}'] = !![]; }}\n")
        }
        (true, false) => {
            format!(
                "    if ({name}.{flag} === undefined) {{ {nested}c = arguments; {name}.{flag} = true; }}\n"
            )
        }
    }
}

fn extras(rng: &mut Rng, shape: &Shape) -> String {
    let mut text = String::new();
    let count = match shape.class {
        Class::Extra => 1 + rng.below(3),
        _ if rng.chance(20) => 1,
        _ => 0,
    };
    for index in 0..count {
        let statement = match rng.below(4) {
            0 => format!("var t{index} = f + 0x{:x};", rng.below(64)),
            1 => format!("var t{index} = e[0x1];"),
            2 => format!("var t{index} = f - e[0x0];"),
            _ => format!("var t{index} = 'k' + f;"),
        };
        let _ = writeln!(text, "    {statement}");
    }
    text
}

fn mutant(rng: &mut Rng, class: Class) -> &'static str {
    match class {
        Class::CallMutant => "    var z = String(f);\n",
        Class::MemberMutant => "    var z = e[0x0].length;\n",
        Class::FunctionMutant => [
            "    function n() { }\n",
            "    function j() { }\n",
            "    function z() { }\n",
        ][rng.below(3)],
        _ => "",
    }
}

fn memo_read(shape: &Shape, decoder: &Decoder) -> String {
    let store = container(shape, decoder);
    if shape.class == Class::WriteMutant {
        return format!(
            "    var j = e[0x0];\n    var n = f + j;\n    j = 0x1;\n    var hit = {store}[n];\n"
        );
    }
    format!("    var j = e[0x0], n = f + j, hit = {store}[n];\n")
}

fn tail(shape: &Shape, decoder: &Decoder) -> String {
    let store = container(shape, decoder);
    if shape.ternary {
        return format!("    return !hit ? ((i = i + '!'), ({store}[n] = i)) : (i = hit), i;\n");
    }
    format!("    var v;\n    if (!hit) {{ v = e[f] + '!'; {store}[n] = v; }} else v = hit;\n    return v;\n")
}

fn decoder_text(rng: &mut Rng, shape: &Shape, decoder: &Decoder) -> String {
    let name = decoder.name;
    let operator = if decoder.shift_up { '+' } else { '-' };
    let offset = offset_expression(rng, decoder.offset);
    let outer = if shape.gate {
        "var e = a();"
    } else {
        "var e = a(), m = {};"
    };
    let update = format!("    f = f {operator} ({offset});\n");
    let mapped = shape.class == Class::MappedParams;
    let mut body = if mapped {
        String::new()
    } else {
        update.clone()
    };
    if shape.ternary {
        body.push_str("    var i = e[f];\n");
    }
    body.push_str(&extras(rng, shape));
    if shape.out_of_range {
        body.push_str("    var r = e[g];\n");
    }
    body.push_str(mutant(rng, shape.class));
    body.push_str(&gate(rng, shape, decoder));
    if mapped {
        body.push_str(&update);
    }
    body.push_str(&memo_read(shape, decoder));
    body.push_str(&tail(shape, decoder));
    format!(
        "function {name}(c, d) {{\n  {outer}\n  return ({name} = function (f, g) {{\n{body}  }}), {name}(c, d);\n}}\n"
    )
}

struct Call {
    text: String,
    value: Option<i64>,
}

struct Terms<'s> {
    array: &'s Array,
    decoders: &'s [Decoder],
    wrappers: Vec<String>,
    shape: &'s Shape,
    calls: Vec<String>,
}

impl Terms<'_> {
    fn call(&mut self, rng: &mut Rng, decoder: usize, index: usize) -> Call {
        let target = &self.decoders[decoder];
        let argument = target.argument(index);
        let second = if self.shape.out_of_range {
            let past = rng.chance(40);
            let g = if past {
                self.array.items.len() + rng.below(50)
            } else {
                rng.below(self.array.items.len())
            };
            format!(", {g}")
        } else {
            String::new()
        };
        let direct = format!("{}({}{second})", target.name, number(rng, argument));
        self.calls.push(direct.clone());
        let wrapped = self.shape.class == Class::Wrapped || rng.chance(15);
        let text = if wrapped && second.is_empty() {
            self.wrapper(rng, target, argument)
        } else {
            direct
        };
        Call {
            text,
            value: self.array.at(index, self.array.rotation),
        }
    }

    fn wrapper(&mut self, rng: &mut Rng, target: &Decoder, argument: i64) -> String {
        let id = self.wrappers.len();
        let shift = rng.between(-0x300, 0x300);
        let position = rng.below(3);
        let params: Vec<String> = (0..3).map(|slot| format!("p{id}x{slot}")).collect();
        let mapped = if shift < 0 {
            format!("{} + {}", params[position], number(rng, -shift))
        } else {
            format!("{} - {}", params[position], number(rng, shift))
        };
        self.wrappers.push(format!(
            "  function w{id}({}) {{ return {}({mapped}); }}\n",
            params.join(", "),
            target.name
        ));
        let mut args: Vec<String> = (0..3)
            .map(|_| {
                let value = rng.between(0, 0x500);
                number(rng, value)
            })
            .collect();
        args[position] = number(rng, argument + shift);
        format!("w{id}({})", args.join(", "))
    }
}

fn parse(call: &Call) -> (String, Option<f64>) {
    (
        format!("parseInt({})", call.text),
        call.value.map(|value| value as f64),
    )
}

fn term(
    rng: &mut Rng,
    terms: &mut Terms<'_>,
    decoder: usize,
    index: usize,
) -> (String, Option<f64>) {
    let (first, value) = parse(&terms.call(rng, decoder, index));
    match rng.below(3) {
        0 => (first, value),
        1 => {
            let k = 1 + rng.below(12);
            (
                format!("-{first} / 0x{k:x}"),
                value.map(|value| (-value) / k as f64),
            )
        }
        _ => {
            let other = terms.array.live(rng);
            let (second, right) = parse(&terms.call(rng, decoder, other));
            let (k1, k2) = (1 + rng.below(9), 1 + rng.below(9));
            let product = value
                .zip(right)
                .map(|(a, b)| (a / k1 as f64) * ((-b) / k2 as f64));
            (format!("{first} / {k1} * (-{second} / {k2})"), product)
        }
    }
}

fn checksum(rng: &mut Rng, terms: &mut Terms<'_>) -> (String, f64) {
    let count = 2 + rng.below(4);
    let mut parts = Vec::new();
    let mut total = 0.0;
    for position in 0..count {
        let opening = (position == 0).then(|| terms.array.opening()).flatten();
        let index = opening.unwrap_or_else(|| terms.array.live(rng));
        let decoder = match terms.shape.class {
            Class::GateUnfired if position > 0 => 1,
            _ => 0,
        };
        let (text, value) = term(rng, terms, decoder, index);
        total = if position == 0 {
            value.unwrap_or(0.0)
        } else {
            total + value.unwrap_or(0.0)
        };
        parts.push(text);
    }
    (parts.join(" + "), total)
}

fn literal_number(value: f64) -> String {
    if value == value.trunc() && value.abs() < 1e15 {
        return format!("{}", value as i64);
    }
    format!("{value:?}")
}

pub fn program(rng: &mut Rng, class: Class) -> String {
    let mut shape = shape(rng, class);
    let array = Array::generate(rng);
    if class == Class::UserPrototype {
        shape.prototype = Some(array.prototype_keys());
    }
    let decoders: Vec<Decoder> = [("b", "k"), ("p", "q")]
        .into_iter()
        .map(|(name, flag)| Decoder {
            name,
            flag,
            offset: rng.between(-0x400, 0x400),
            shift_up: rng.chance(30),
        })
        .collect();
    let mut terms = Terms {
        array: &array,
        decoders: &decoders,
        wrappers: Vec::new(),
        shape: &shape,
        calls: Vec::new(),
    };
    let (sum, total) = checksum(rng, &mut terms);
    let mut text = format!(
        "function a() {{ var h = [{}]; a = function () {{ return h; }}; return a(); }}\n",
        array.literal()
    );
    let used = if class == Class::GateUnfired { 2 } else { 1 };
    for decoder in &decoders[..used] {
        text.push_str(&decoder_text(rng, &shape, decoder));
    }
    let _ = write!(
        text,
        "(function (c, d) {{\n  var e = c();\n{}  while (!![]) {{\n    try {{\n      var f = {sum};\n      if (f === d) break;\n      else e['push'](e['shift']());\n    }} catch (h) {{ e['push'](e['shift']()); }}\n  }}\n}})(a, {});\n",
        terms.wrappers.concat(),
        literal_number(total)
    );
    let _ = writeln!(text, "console.log({});", terms.calls.join(", "));
    text
}
