use oxc_allocator::Allocator;
use untwist_core::pass::PassOptions;

fn run_mode(source: &str, extended: bool) -> String {
    let alloc = Allocator::default();
    let (code, report) = untwist_testing::transform_report(
        &alloc,
        source,
        &[untwist_deobfuscate::merge_object_assignments::boxed],
        PassOptions {
            extended,
            ..PassOptions::default()
        },
    );
    assert!(report.converged(), "{:?}", report.stop);
    code
}

fn assigns_a_member_of_o(code: &str) -> bool {
    code.lines().any(|line| {
        let line = line.trim_start();
        (line.starts_with("o.") || line.starts_with("o[")) && line.contains(" = ")
    })
}

fn assert_rows(rows: &[(&str, &str)], folds: bool) {
    for (name, source) in rows {
        for extended in [true, false] {
            let code = run_mode(source, extended);
            assert_eq!(
                !assigns_a_member_of_o(&code),
                folds,
                "{name} extended={extended}: {code}"
            );
            untwist_testing::node::assert_node_agrees(source, &code);
        }
    }
}

const ALLOWED: &[(&str, &str)] = &[
    (
        "a member read and arithmetic on parameters",
        "function f(x, p, q) {\n  var o = {};\n  o.a = x.y;\n  o.b = p * q;\n  return o;\n}\nconsole.log(JSON.stringify(f({ y: 1 }, 2, 3)));",
    ),
    (
        "a getter that throws leaves the function with the object dead",
        "function f(x) {\n  var o = {};\n  o.a = 1;\n  o.b = x.y;\n  return o;\n}\ntry {\n  f({ get y() { throw new Error(\"boom\"); } });\n} catch (e) {\n  console.log(e.message);\n}",
    ),
    (
        "getters and valueOf keep their order",
        "var log = [];\nvar x = { get y() { log.push(\"y\"); return 1; } };\nvar p = { valueOf() { log.push(\"p\"); return 2; } };\nfunction f() {\n  const o = {};\n  o.a = x.y;\n  o.b = p + 1;\n  o.c = x.y;\n  return o;\n}\nconsole.log(JSON.stringify(f()), log.join());",
    ),
    (
        "this member reads in a method",
        "var r = {\n  width: 2,\n  height: 3,\n  scale(k) {\n    const o = {};\n    o.width = this.width * k;\n    o.height = this.height * k;\n    return o;\n  }\n};\nconsole.log(JSON.stringify(r.scale(2)));",
    ),
    (
        "a let in a block of the function and a template",
        "function f(x) {\n  if (x) {\n    let o = {};\n    o.a = x.k;\n    o.b = `${x.k}!`;\n    console.log(JSON.stringify(o));\n  }\n}\nf({ k: \"v\" });",
    ),
    (
        "an arrow body with computed reads, unary and typeof",
        "const f = (x) => {\n  const o = {};\n  o.a = x[0];\n  o.b = -x[1];\n  o.c = typeof x.z;\n  o.d = x[0] < x[1] ? x[0] : x[1];\n  return o;\n};\nconsole.log(JSON.stringify(f([1, 2])));",
    ),
    (
        "a read of a let before its declaration throws out of the function",
        "function f() {\n  const o = {};\n  o.a = 1;\n  o.b = later;\n  let later = 2;\n  return o;\n}\ntry {\n  f();\n} catch (e) {\n  console.log(e.name);\n}",
    ),
    (
        "a read of an undeclared name throws out of the function",
        "function f() {\n  var o = {};\n  o.a = 1;\n  o.b = missing.q;\n  return o;\n}\ntry {\n  f();\n} catch (e) {\n  console.log(e.name);\n}",
    ),
    (
        "a try outside the function does not count",
        "function f(x) {\n  var o = {};\n  o.a = 1;\n  o.b = x.y;\n  return o;\n}\ntry {\n  console.log(JSON.stringify(f({ y: 2 })));\n  f(null);\n} catch (e) {\n  console.log(e.name);\n}",
    ),
    (
        "a var in a block read again inside a nested block",
        "function f(list) {\n  var out = [];\n  for (var i = 0; i < list.length; i++) {\n    var o = {};\n    o.a = list[i].v;\n    o.b = list[i].v * 2;\n    if (o.a > 1) {\n      out.push(o);\n    }\n  }\n  return out;\n}\nconsole.log(JSON.stringify(f([{ v: 1 }, { v: 3 }])));",
    ),
    (
        "a closure whose own binding shadows the name",
        "function f(x) {\n  var o = {};\n  o.a = x.y;\n  o.b = x.y + 1;\n  var ids = [3].map(function (o) {\n    return o;\n  });\n  return [o, ids];\n}\nconsole.log(JSON.stringify(f({ y: 1 })));",
    ),
    (
        "a var in a loop block whose every reference is in that block",
        "function f(list) {\n  var out = [];\n  for (var i = 0; i < list.length; i++) {\n    var o = {};\n    o.a = list[i].v;\n    o.b = list[i].v * 2;\n    out.push(o);\n  }\n  return out;\n}\nconsole.log(JSON.stringify(f([{ v: 1 }, { v: 2 }])));",
    ),
];

const REFUSED: &[(&str, &str)] = &[
    (
        "a var under try with a catch that reads it",
        "function f(x) {\n  try {\n    var o = {};\n    o.a = 1;\n    o.b = x.y;\n  } catch (e) {\n    console.log(o.a);\n  }\n}\nf({ get y() { throw 1; } });",
    ),
    (
        "a var under try with a finally that reads it",
        "function f(x) {\n  try {\n    var o = {};\n    o.a = 1;\n    o.b = x.y;\n  } finally {\n    console.log(JSON.stringify(o));\n  }\n}\ntry {\n  f({ get y() { throw 1; } });\n} catch (e) {\n  console.log(e);\n}",
    ),
    (
        "a let in a try block",
        "function f(x) {\n  try {\n    let o = {};\n    o.a = 1;\n    o.b = x.y;\n    console.log(JSON.stringify(o));\n  } catch (e) {\n    console.log(\"caught\");\n  }\n}\nf({ y: 2 });",
    ),
    (
        "a getter that names the object",
        "function n() {\n  var holder = { get q() { return JSON.stringify(o); } };\n  var o = {};\n  o.a = 1;\n  o.b = holder.q;\n  console.log(o.b);\n}\nn();",
    ),
    (
        "a closure the getter calls",
        "function n() {\n  var peek = () => o;\n  var holder = { get q() { return peek().a; } };\n  var o = {};\n  o.a = 1;\n  o.b = holder.q;\n  console.log(o.b);\n}\nn();",
    ),
    (
        "a class field initialiser that reads the object",
        "function n() {\n  class C {\n    v = JSON.stringify(o);\n  }\n  var x = { get y() { return new C().v; } };\n  var o = {};\n  o.a = 1;\n  o.b = x.y;\n  console.log(o.b);\n}\nn();",
    ),
    (
        "a hoisted function declaration handed out before the run",
        "function n(x) {\n  x.peek = peek;\n  var o = {};\n  o.a = 1;\n  o.b = x.y;\n  return o.b;\n  function peek() {\n    return o.a;\n  }\n}\nconsole.log(n({ get y() { return this.peek(); } }));",
    ),
    (
        "a script-level var",
        "var x = { get y() { return globalThis.o ? globalThis.o.a : \"module\"; } };\nvar o = {};\no.a = 1;\no.b = x.y;\nconsole.log(o.b);",
    ),
    (
        "a script-level const an indirect eval can name",
        "var x = { get y() { try { return (0, eval)(\"o\").a; } catch (e) { return e.name; } } };\nconst o = {};\no.a = 1;\no.b = x.y;\nconsole.log(o.b);",
    ),
    (
        "a computed key with a value that may run code",
        "function f(x, k) {\n  var o = {};\n  o[k] = x.y;\n  return o;\n}\nvar log = [];\nvar r = f({ get y() { log.push(\"y\"); return 1; } }, { toString() { log.push(\"k\"); return \"a\"; } });\nconsole.log(JSON.stringify(r), log.join());",
    ),
    (
        "a value that reads the object",
        "function f() {\n  var o = {};\n  o.a = 1;\n  o.b = o.a + 1;\n  return o;\n}\nconsole.log(JSON.stringify(f()));",
    ),
    (
        "in and instanceof",
        "function f(x) {\n  var o = {};\n  o.a = \"k\" in x;\n  o.b = x instanceof Object;\n  return o;\n}\nconsole.log(JSON.stringify(f({ k: 1 })));",
    ),
    (
        "an optional member read",
        "function f(x) {\n  var o = {};\n  o.a = x?.y;\n  return o;\n}\nconsole.log(JSON.stringify(f({ y: 1 })));",
    ),
    (
        "a var in a nested block",
        "function f(x) {\n  if (x) {\n    var o = {};\n    o.a = x.k;\n  }\n  return o;\n}\nconsole.log(JSON.stringify(f({ k: 1 })));",
    ),
    (
        "a var in a block that a closure outside the block names",
        "function f(x) {\n  x.peek = () => o.a;\n  if (x) {\n    var o = {};\n    o.a = 1;\n    o.b = x.y;\n    return o.b;\n  }\n}\nconsole.log(f({ get y() { try { return this.peek(); } catch (e) { return e.name; } } }));",
    ),
    (
        "a class static block",
        "class C {\n  static k = 2;\n  static {\n    var o = {};\n    o.a = C.k;\n    o.b = C.k * 2;\n    console.log(JSON.stringify(o));\n  }\n}",
    ),
    (
        "a var that redeclares a parameter",
        "function f(o, x) {\n  var o = {};\n  o.a = 1;\n  o.b = x.y;\n  return o;\n}\nconsole.log(JSON.stringify(f(0, { y: 2 })));",
    ),
    (
        "a with statement around the run",
        "function f(w, x) {\n  with (w) {\n    var o = {};\n    o.a = x.y;\n  }\n  return o;\n}\nconsole.log(JSON.stringify(f({}, { y: 1 })));",
    ),
    (
        "a const in a switch case that an earlier case's closure names",
        "function f(k, x) {\n  switch (k) {\n    case 0:\n      x.peek = () => o.a;\n    case 1:\n      const o = {};\n      o.a = 1;\n      o.b = x.y;\n      return o.b;\n  }\n}\nconsole.log(f(0, { get y() { try { return this.peek(); } catch (e) { return e.name; } } }));",
    ),
    (
        "an assignment inside the value",
        "function f(x) {\n  var t;\n  var o = {};\n  o.a = (t = x.y);\n  return [o, t];\n}\nconsole.log(JSON.stringify(f({ y: 1 })));",
    ),
];

#[test]
fn member_reads_and_arithmetic_fold_into_a_function_local_uncaptured_object() {
    assert_rows(ALLOWED, true);
}

#[test]
fn member_reads_and_arithmetic_stay_out_where_the_object_could_be_observed() {
    assert_rows(REFUSED, false);
}
