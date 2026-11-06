use untwist::{Options, Output, Pipeline};
use untwist_testing::node_rows::{mismatches, node_or_skip, patched_after_rotator, Mode};

const MACHINERY: &str = "function a(";

fn safe_lane(source: &str) -> Output {
    Pipeline::new(Options {
        unpack: false,
        jsx: false,
        ..Options::default()
    })
    .run(source)
}

fn machinery_removed(source: &str) -> bool {
    !safe_lane(source).code().contains(MACHINERY)
}

const BOUND_SHAPES: [(&str, &str); 3] = [
    (
        "a getter and a method called through the literal's binding",
        "const shape = {\nwidth: 6,\nheight: 4,\nget area() {\nreturn this.width * this.height;\n},\nscale(factor) {\nreturn { width: this.width * factor, height: this.height * factor };\n},\n};\nconst scaled = shape.scale(3);\nconsole.log(shape.area, scaled.width, scaled.height);",
    ),
    (
        "a method that calls another through this",
        "const counter = {\nn: 2,\ntwice() {\nreturn this.double() + this.double();\n},\ndouble() {\nreturn this.n * 2;\n},\n};\nconsole.log(counter.twice());",
    ),
    (
        "a setter written through the literal's binding",
        "const box = {\nstore: 1,\nget v() {\nreturn this.store;\n},\nset v(x) {\nthis.store = x;\n},\n};\nbox.v = 5;\nconsole.log(box.v);",
    ),
];

const LEAKED_SHAPES: [(&str, &str); 9] = [
    (
        "a method read as a value",
        "const o = {\nm() {\nreturn this.w;\n},\n};\nconst f = o.m;\nconsole.log(o.m(), typeof f);",
    ),
    (
        "a method called through call",
        "const o = {\nm() {\nreturn this.w;\n},\n};\nconsole.log(o.m(), o.m.call(undefined));",
    ),
    (
        "a method destructured out of the literal",
        "const o = {\nm() {\nreturn this.w;\n},\n};\nconst { m } = o;\nconsole.log(o.m(), typeof m);",
    ),
    (
        "a literal spread into another object",
        "const o = {\nm() {\nreturn this.w;\n},\n};\nconst c = { ...o };\nconsole.log(o.m(), typeof c.m);",
    ),
    (
        "a method read by an unknown key",
        "const o = {\nm() {\nreturn this.w;\n},\n};\nconsole.log(o.m(), typeof o[String.fromCharCode(109)]);",
    ),
    (
        "a method that hands out another through this",
        "const o = {\nm() {\nreturn this.w;\n},\nleak() {\nreturn this.m;\n},\n};\nconsole.log(o.m(), typeof o.leak());",
    ),
    (
        "a literal bound by a top-level var",
        "var o = {\nm() {\nreturn this.w;\n},\n};\nconsole.log(o.m(), typeof globalThis[\"o\"].m);",
    ),
    (
        "a named function value that calls itself",
        "const o = {\nm: function self(d) {\nreturn d ? this.w : self(1);\n},\n};\nconsole.log(o.m(0));",
    ),
    (
        "a method that calls itself through arguments.callee",
        "const o = {\nm(d) {\nreturn d ? this.w : arguments.callee(1);\n},\n};\nconsole.log(o.m(0));",
    ),
];

#[test]
fn this_in_a_method_only_its_literal_calls_is_not_the_global_object() {
    for (label, body) in BOUND_SHAPES {
        let source = patched_after_rotator("true", body);
        assert!(
            machinery_removed(&source),
            "{label}:\n{}",
            safe_lane(&source).code()
        );
    }
}

#[test]
fn this_in_a_method_that_may_run_unbound_still_keeps_the_machinery() {
    for (label, body) in LEAKED_SHAPES {
        let source = patched_after_rotator("true", body);
        assert!(
            !machinery_removed(&source),
            "{label}:\n{}",
            safe_lane(&source).code()
        );
    }
}

#[test]
fn every_bound_shape_prints_what_node_prints_after_deobfuscation() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let fixtures: Vec<(String, String)> = BOUND_SHAPES
        .iter()
        .map(|(label, body)| (label.to_string(), patched_after_rotator("true", body)))
        .collect();
    let lane = |source: &str| safe_lane(source).code().to_string();
    let runs: [Mode<'_>; 1] = [("safe", &lane)];
    let wrong = mismatches(&node, &fixtures, &runs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
