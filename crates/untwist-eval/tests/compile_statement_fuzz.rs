use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::interp::{eval_script_decoding, eval_script_in};
use untwist_eval::test_hooks::{compile_census, record_units_charged, UnitRecord};

struct Gen {
    state: u64,
    counter: u32,
    functions: Vec<String>,
}

#[derive(Clone, Copy)]
struct Ctx {
    in_loop: bool,
    in_switch: bool,
    top: bool,
}

const LOCALS: [&str; 8] = ["a", "b", "c", "v0", "v1", "v2", "v3", "arguments"];

impl Gen {
    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }
    fn pick(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn fresh(&mut self) -> u32 {
        self.counter += 1;
        self.counter
    }
    fn ident(&mut self) -> String {
        match self.pick(10) {
            0 => "o".to_string(),
            1 => "r".to_string(),
            2 => "undefx".to_string(),
            3 if !self.functions.is_empty() => {
                let i = self.pick(self.functions.len() as u64) as usize;
                self.functions[i].clone()
            }
            _ => LOCALS[self.pick(LOCALS.len() as u64 - 1) as usize].to_string(),
        }
    }
    fn target(&mut self) -> String {
        match self.pick(6) {
            0 => "o.p".to_string(),
            1 => format!("o[{}]", self.leaf()),
            2 => "r".to_string(),
            _ => LOCALS[self.pick(7) as usize].to_string(),
        }
    }
    fn leaf(&mut self) -> String {
        match self.pick(8) {
            0 => format!("{}", self.pick(4)),
            1 => "'s'".to_string(),
            2 => "'q'".to_string(),
            3 => "null".to_string(),
            4 => "true".to_string(),
            _ => self.ident(),
        }
    }
    fn array(&mut self, d: u32) -> String {
        let n = self.pick(5);
        let items: Vec<String> = (0..n)
            .map(|_| match self.pick(12) {
                0 => String::new(),
                1 => format!("...{}", self.expr(d)),
                2..=6 => self.leaf(),
                _ => self.expr(d),
            })
            .collect();
        format!("[{}]", items.join(", "))
    }
    fn expr(&mut self, depth: u32) -> String {
        if depth == 0 {
            return self.leaf();
        }
        let d = depth - 1;
        const OPS: [&str; 12] = [
            "+", "-", "*", "%", "===", "!==", "<", "&", "|", "^", ">>", "**",
        ];
        const COMPOUND: [&str; 8] = ["+=", "-=", "*=", "|=", "^=", "<<=", ">>>=", "**="];
        match self.pick(22) {
            0..=2 => {
                let op = OPS[self.pick(OPS.len() as u64) as usize];
                format!("({} {op} {})", self.expr(d), self.expr(d))
            }
            3 => format!("({} && {})", self.expr(d), self.expr(d)),
            4 => format!("({} || {})", self.expr(d), self.expr(d)),
            5 => format!("({} ? {} : {})", self.expr(d), self.expr(d), self.expr(d)),
            6 => format!("({} = {})", self.target(), self.expr(d)),
            7 | 8 => {
                let op = COMPOUND[self.pick(COMPOUND.len() as u64) as usize];
                format!("({} {op} {})", self.target(), self.expr(d))
            }
            9 => format!("({}++)", self.target()),
            10 => self.array(d),
            11 => "this".to_string(),
            12 => format!("(new F({}))", self.expr(d)),
            13 => format!("(new F({}).x)", self.expr(d)),
            14 if !self.functions.is_empty() => {
                let i = self.pick(self.functions.len() as u64) as usize;
                format!("{}({})", self.functions[i].clone(), self.expr(d))
            }
            15 if d > 0 => {
                let body = self.block(
                    d.min(2),
                    Ctx {
                        in_loop: false,
                        in_switch: false,
                        top: false,
                    },
                );
                format!("(function () {{ {body} }})()")
            }
            16 => format!("(typeof {})", self.ident()),
            17 => format!("({}.p)", self.expr(d)),
            18 => format!("({}[{}])", self.expr(d), self.expr(d)),
            19 => format!("(!{})", self.expr(d)),
            20 => format!("[{}, {}].length", self.leaf(), self.leaf()),
            _ => format!("({}, {})", self.expr(d), self.expr(d)),
        }
    }
    fn block(&mut self, depth: u32, ctx: Ctx) -> String {
        let n = 1 + self.pick(3);
        (0..n)
            .map(|_| self.stmt(depth, ctx))
            .collect::<Vec<_>>()
            .join(" ")
    }
    fn stmt(&mut self, depth: u32, ctx: Ctx) -> String {
        let inner = Ctx { top: false, ..ctx };
        if depth == 0 {
            return format!("{};", self.expr(2));
        }
        let d = depth - 1;
        match self.pick(20) {
            0 | 1 => format!("{};", self.expr(2)),
            2 => {
                let v = LOCALS[3 + self.pick(4) as usize];
                format!("var {v} = {};", self.expr(2))
            }
            3 => {
                let v = LOCALS[3 + self.pick(4) as usize];
                format!("var {v};")
            }
            4 => format!("if ({}) {{ {} }}", self.expr(2), self.block(d, inner)),
            5 => format!(
                "if ({}) {} else {}",
                self.expr(2),
                self.stmt(d, inner),
                self.stmt(d, inner)
            ),
            6 => format!("{{ {} }}", self.block(d, inner)),
            7..=11 => self.stmt_control(d, inner),
            12 if ctx.in_loop || ctx.in_switch => "break;".to_string(),
            13 if ctx.in_loop => "continue;".to_string(),
            14 => format!("if ({}) return {};", self.expr(1), self.expr(2)),
            15 if ctx.top => {
                let k = self.fresh();
                let name = format!("g{k}");
                self.functions.push(name.clone());
                let body = self.block(
                    d.min(2),
                    Ctx {
                        in_loop: false,
                        in_switch: false,
                        top: true,
                    },
                );
                format!("function {name}(x) {{ {body} return x; }}")
            }
            16 => format!(
                "v{} = function (y) {{ return {}; }};",
                self.pick(4),
                self.expr(2)
            ),
            _ => format!("{};", self.expr(2)),
        }
    }
    fn stmt_control(&mut self, d: u32, inner: Ctx) -> String {
        match self.pick(5) {
            0 => {
                let k = self.fresh();
                let body = self.block(
                    d,
                    Ctx {
                        in_loop: true,
                        ..inner
                    },
                );
                format!(
                    "for (var k{k} = 0; k{k} < {}; k{k}++) {{ {body} }}",
                    self.pick(4)
                )
            }
            1 => {
                let k = self.fresh();
                let body = self.block(
                    d,
                    Ctx {
                        in_loop: true,
                        ..inner
                    },
                );
                format!(
                    "var w{k} = {}; while (w{k}-- > 0) {{ {body} }}",
                    self.pick(4)
                )
            }
            2 => {
                let body = self.block(d, inner);
                let handler = self.block(d, inner);
                match self.pick(3) {
                    0 => format!("try {{ {body} }} catch {{ {handler} }}"),
                    1 => format!("try {{ {body} }} catch (v1) {{ {handler} }}"),
                    _ => format!("try {{ {body} }} catch (e) {{ v0 = e; {handler} }}"),
                }
            }
            3 => format!("if ({}) throw {};", self.expr(1), self.expr(2)),
            _ => self.switch(d, inner),
        }
    }
    fn switch(&mut self, d: u32, inner: Ctx) -> String {
        let cases: Vec<String> = (0..1 + self.pick(4))
            .map(|_| {
                let body = self.block(
                    d.min(1),
                    Ctx {
                        in_switch: true,
                        ..inner
                    },
                );
                match self.pick(5) {
                    0 => format!("default: {body}"),
                    _ => format!("case {}: {body}", self.leaf()),
                }
            })
            .collect();
        let mut seen_default = false;
        let cases: Vec<String> = cases
            .into_iter()
            .filter(|c| {
                if c.starts_with("default") {
                    if seen_default {
                        return false;
                    }
                    seen_default = true;
                }
                true
            })
            .collect();
        format!("switch ({}) {{ {} }}", self.expr(1), cases.join(" "))
    }
    fn program(&mut self) -> String {
        self.functions.clear();
        let strict = if self.pick(5) == 0 {
            "'use strict'; "
        } else {
            ""
        };
        let body = self.block(
            3,
            Ctx {
                in_loop: false,
                in_switch: false,
                top: true,
            },
        );
        let body = format!(
            "{body} {}",
            self.block(
                3,
                Ctx {
                    in_loop: false,
                    in_switch: false,
                    top: true
                }
            )
        );
        format!(
            "var o = {{p: 1, q: 'z'}}, r = 0; function F(x) {{ this.x = x; }}\nfunction main(a, b, c) {{ {strict}{body} return [a, b, c, v0, v1, v2, v3]; var v0, v1, v2, v3; }}\nvar out = main(1, 's', 2) + '|' + main(0, '', null) + '|' + main.call(o, o); out + '|' + o.p + '|' + r;"
        )
    }
}

fn run(source: &str, decoding: bool, fuel: u64) -> (Vec<UnitRecord>, u64) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    if !parsed.diagnostics.is_empty() {
        return (Vec::new(), u64::MAX);
    }
    let program = &parsed.program;
    let go = || match decoding {
        true => eval_script_decoding(program, Budget::new(fuel), |_| ()),
        false => eval_script_in(program, Budget::new(fuel), |_, _| ()),
    };
    let mut records = Vec::new();
    let census = compile_census(|| records = record_units_charged(go));
    (records, census.calls)
}

#[derive(Default)]
struct Tally {
    runs: u64,
    calls: u64,
}

const FULL_FUEL: u64 = 2_000_000;
const CAPS_PER_PROGRAM: usize = 3;

fn fuels(gen: &mut Gen, full: &[UnitRecord]) -> Vec<u64> {
    let steps = full.first().map_or(0, |record| record.steps_used);
    let mut fuels = vec![FULL_FUEL];
    for _ in 0..CAPS_PER_PROGRAM {
        fuels.push(gen.pick(steps + 2));
    }
    fuels
}

fn compare(gen: &mut Gen, source: &str, decoding: bool, tally: &mut Tally) {
    let (full, calls) = run(source, decoding, FULL_FUEL);
    if calls == u64::MAX {
        return;
    }
    let mut runs = Vec::new();
    for (index, fuel) in fuels(gen, &full).into_iter().enumerate() {
        let (compiled, calls) = match index {
            0 => (full.clone(), calls),
            _ => run(source, decoding, fuel),
        };
        runs.push((fuel, compiled));
        tally.runs += 1;
        tally.calls += calls;
    }
    let fuels: Vec<u64> = runs.iter().map(|(fuel, _)| *fuel).collect();
    untwist_eval::test_hooks::frozen(
        "statement fuzz",
        &format!("{decoding}\n{fuels:?}\n{source}"),
        std::slice::from_ref(&runs),
    );
}

const PROGRAMS: u64 = 2_000;
const SEEDS: [u64; 3] = [0x9e37_79b9_7f4a_7c15, 2, 3];

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 6,000 generated statement programs, each in both modes and at four fuel caps"
)]
fn generated_statement_programs_run_compiled_as_frozen_with_the_ledger_and_fuel_caps() {
    let mut tally = Tally::default();
    for seed in SEEDS {
        let mut gen = Gen {
            state: seed,
            counter: 0,
            functions: Vec::new(),
        };
        for _ in 0..PROGRAMS {
            let source = gen.program();
            for decoding in [false, true] {
                compare(&mut gen, &source, decoding, &mut tally);
            }
        }
    }
    println!(
        "statement fuzz: {} programs, {} runs, {} compiled calls, all as frozen",
        PROGRAMS * SEEDS.len() as u64,
        tally.runs,
        tally.calls
    );
    assert!(tally.calls > 0);
}
