use untwist::{Options, Pipeline};

fn run(source: &str) -> String {
    let options = Options {
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

#[test]
fn a_boolean_member_does_not_pick_the_wrong_eval_branch() {
    let out = run(
        "eval(typeof true.toString === \"undefined\" ? \"console.log('wrong')\" : \"console.log('right')\");\n",
    );
    assert!(!out.contains("wrong") || out.contains("right"), "{out}");
    assert!(!out.contains("console.log(\"wrong\")"), "{out}");
}

#[test]
fn a_boolean_constructor_is_not_read_as_undefined_in_a_layer() {
    let out =
        run("eval(`console.log(${typeof (false).constructor === \"undefined\" ? 1 : 2})`);\n");
    assert!(!out.contains("console.log(1)"), "{out}");
}

#[test]
fn a_boolean_method_is_not_read_as_falsy_in_a_layer() {
    let out = run("eval(\"console.log(\" + (true.valueOf ? 1 : 2) + \")\");\n");
    assert!(!out.contains("console.log(2)"), "{out}");
}

#[test]
fn unmodelled_intrinsic_members_are_not_printed_as_undefined() {
    let reads = [
        "Math.SQRT2",
        "Math.abs.length",
        "({a:1}).constructor",
        "String.name",
        "true.toString",
    ];
    for read in reads {
        let out = run(&format!(
            "eval(\"console.log(1, '\" + typeof ({read}) + \"|\" + ({read}) + \"')\");\n"
        ));
        assert!(!out.contains("undefined"), "{read}: {out}");
    }
}
