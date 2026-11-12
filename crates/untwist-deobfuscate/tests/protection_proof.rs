use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::pass::PassOptions;
use untwist_core::print::{print, PrintStyle};

const CONTROLLER: &str = "var ctl = (function () {\n  var first = true;\n  return function (ctx, fn) {\n    var rfn = first ? function () {\n      if (fn) {\n        var res = fn.apply(ctx, arguments);\n        fn = null;\n        return res;\n      }\n    } : function () {};\n    first = false;\n    return rfn;\n  };\n})();\n";

const A_PAYLOAD_RUN_THROUGH_THE_CONTROLLER: &str = "var run = ctl(this, function () {\n  console.log(\"payload one\");\n  globalThis.marker = 1;\n});\nrun();\nctl(this, function () {\n  console.log(\"payload two\");\n})();\nconsole.log(globalThis.marker);";

const THE_SOURCE_CHECK: &str = "var sd = ctl(this, function () {\n  return sd.toString().search(\"(((.+)+)+)+$\").toString().constructor(sd).search(\"(((.+)+)+)+$\");\n});\nsd();\n";

const THE_CONSOLE_DISABLE: &str = "var cd = ctl(this, function () {\n  let that;\n  try {\n    const getGlobal = Function(\"return (function() {}.constructor(\\\"return this\\\")( ));\");\n    that = getGlobal();\n  } catch (e) {\n    that = window;\n  }\n  const consoleObject = that.console = that.console || {};\n  const methods = [\"log\", \"warn\", \"info\", \"error\", \"exception\", \"table\", \"trace\"];\n  for (let index = 0; index < methods.length; index++) {\n    const func = ctl.constructor.prototype.bind(ctl);\n    const methodName = methods[index];\n    const originalFunction = consoleObject[methodName] || func;\n    func.__proto__ = ctl.bind(ctl);\n    func.toString = originalFunction.toString.bind(originalFunction);\n    consoleObject[methodName] = func;\n  }\n});\ncd();\n";

const THE_DEBUG_PROTECTION: &str = "function dpf(ret) {\n  function debuggerProtection(counter) {\n    if (typeof counter === \"string\") {\n      return function (arg) {}.constructor(\"while (true) {}\").apply(\"counter\");\n    } else if ((\"\" + counter / counter).length !== 1 || counter % 20 === 0) {\n      (function () {\n        return true;\n      }).constructor(\"debugger\").call(\"action\");\n    } else {\n      (function () {\n        return false;\n      }).constructor(\"debugger\").apply(\"stateObject\");\n    }\n    debuggerProtection(++counter);\n  }\n  try {\n    if (ret) {\n      return debuggerProtection;\n    } else {\n      debuggerProtection(0);\n    }\n  } catch (y) {}\n}\n";

const THE_DEBUG_PROTECTION_START: &str = "(function () {\n  ctl(this, function () {\n    const first = new RegExp(\"function *\\\\( *\\\\)\");\n    const second = new RegExp(\"\\\\+\\\\+ *(?:[a-zA-Z_$][0-9a-zA-Z_$]*)\", \"i\");\n    const result = dpf(\"init\");\n    if (!first.test(result + \"chain\") || !second.test(result + \"input\")) {\n      result(\"0\");\n    } else {\n      dpf();\n    }\n  })();\n})();\n";

const REST: &str = "console.log(\"x\");";

fn run_lane<'a>(alloc: &'a Allocator, source: &'a str, safe: bool) -> (String, Vec<String>) {
    let (code, report) = untwist_testing::transform_report(
        alloc,
        source,
        &untwist_deobfuscate::protections_ctors(),
        PassOptions {
            unsafe_transforms: !safe,
            ..PassOptions::default()
        },
    );
    let notes = report.notes.into_iter().map(|note| note.message).collect();
    (code, notes)
}

fn safe(source: &str) -> String {
    let alloc = Allocator::default();
    run_lane(&alloc, source, true).0
}

fn printed(source: &str) -> String {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    print(&parsed.program, PrintStyle::Pretty)
}

#[test]
fn a_payload_run_through_a_controller_shaped_helper_is_kept() {
    let source = format!("{CONTROLLER}{A_PAYLOAD_RUN_THROUGH_THE_CONTROLLER}");
    let alloc = Allocator::default();
    let (out, notes) = run_lane(&alloc, &source, true);
    assert_eq!(out, printed(&source));
    assert!(
        notes
            .iter()
            .any(|note| note == untwist_deobfuscate::notes::SELF_DEFENDING_REFERENCE_KEPT),
        "{notes:?}"
    );
    untwist_testing::node::assert_node_agrees(&source, &out);
}

#[test]
fn the_unsafe_lane_still_trusts_the_controller_shape() {
    let source = format!("{CONTROLLER}{A_PAYLOAD_RUN_THROUGH_THE_CONTROLLER}");
    let alloc = Allocator::default();
    let (out, _) = run_lane(&alloc, &source, false);
    assert_eq!(out.trim(), "console.log(globalThis.marker);");
}

#[test]
fn the_source_check_is_removed_with_its_controller() {
    let source = format!("{CONTROLLER}{THE_SOURCE_CHECK}{REST}");
    assert_eq!(safe(&source).trim(), REST);
}

#[test]
fn the_console_disable_is_removed_with_its_controller() {
    let source = format!("{CONTROLLER}{THE_CONSOLE_DISABLE}{REST}");
    assert_eq!(safe(&source).trim(), REST);
}

#[test]
fn the_debug_protection_start_is_removed_with_the_protection_it_calls() {
    let source = format!("{CONTROLLER}{THE_DEBUG_PROTECTION}{THE_DEBUG_PROTECTION_START}{REST}");
    assert_eq!(safe(&source).trim(), REST);
}

#[test]
fn every_protection_of_one_controller_is_removed_together() {
    let source = format!("{CONTROLLER}{THE_SOURCE_CHECK}{THE_CONSOLE_DISABLE}{REST}");
    assert_eq!(safe(&source).trim(), REST);
}

#[test]
fn one_unproven_callback_keeps_every_call_of_its_controller() {
    let source = format!(
        "{CONTROLLER}{THE_SOURCE_CHECK}ctl(this, function () {{\n  console.log(\"payload\");\n}})();\n{REST}"
    );
    assert_eq!(safe(&source), printed(&source));
}

#[test]
fn a_source_check_with_one_more_statement_is_kept() {
    let source = format!(
        "{CONTROLLER}{}{REST}",
        THE_SOURCE_CHECK.replacen(
            "  return sd.",
            "  console.log(\"payload\");\n  return sd.",
            1
        )
    );
    assert_eq!(safe(&source), printed(&source));
}

#[test]
fn a_source_check_of_another_function_is_kept() {
    let source = format!(
        "function other() {{}}\n{CONTROLLER}{}{REST}",
        THE_SOURCE_CHECK.replacen(".constructor(sd)", ".constructor(other)", 1)
    );
    assert_eq!(safe(&source), printed(&source));
}

#[test]
fn a_console_disable_that_reads_a_replaced_global_is_kept() {
    for patch in [
        "Function = function () {\n  console.log(\"payload\");\n  return function () {};\n};\n",
        "window = null;\n",
    ] {
        let source = format!("{patch}{CONTROLLER}{THE_CONSOLE_DISABLE}{REST}");
        assert_eq!(safe(&source), printed(&source), "{patch}");
    }
}

#[test]
fn a_start_callback_for_a_function_that_is_not_the_protection_is_kept() {
    let source = format!(
        "{CONTROLLER}function dpf(ret) {{\n  console.log(\"payload\", ret);\n  return dpf;\n}}\n{THE_DEBUG_PROTECTION_START}{REST}"
    );
    assert_eq!(safe(&source), printed(&source));
}

const THE_CONSOLE_DISABLE_WITH_A_GETTER: &str = "var cd = ctl(this, function () {\n  var getGlobal = function () {\n    var that;\n    try {\n      that = Function(\"return (function() {}.constructor(\\\"return this\\\")( ));\")();\n    } catch (e) {\n      that = window;\n    }\n    return that;\n  };\n  var that = getGlobal();\n  var consoleObject = that.console = that.console || {};\n  var methods = [\"log\", \"warn\", \"info\", \"error\", \"exception\", \"table\", \"trace\"];\n  for (var index = 0; index < methods.length; index++) {\n    var func = ctl.constructor.prototype.bind(ctl);\n    var methodName = methods[index];\n    var originalFunction = consoleObject[methodName] || func;\n    func.__proto__ = ctl.bind(ctl);\n    func.toString = originalFunction.toString.bind(originalFunction);\n    consoleObject[methodName] = func;\n  }\n});\ncd();\n";

const THE_SOURCE_PROBE: &str = "var sp = ctl(this, function () {\n  var dev = function () {\n    return \"dev\";\n  };\n  var win = function () {\n    return \"window\";\n  };\n  var first = function () {\n    var test = new RegExp(\"\\\\w+ *\\\\(\\\\) *{\\\\w+ *['|\\\"].+['|\\\"];? *}\");\n    return !test.test(dev.toString());\n  };\n  var second = function () {\n    var test = new RegExp(\"(\\\\\\\\[x|u](\\\\w){2,4})+\");\n    return test.test(win.toString());\n  };\n  var loop = function (name) {\n    var marker = ~-1 >> 1 + 255 % 0;\n    if (name.indexOf(marker === \"i\")) {\n      back(name);\n    }\n  };\n  var back = function (name) {\n    var marker = ~-4 >> 1 + 255 % 0;\n    if (name.indexOf((true + \"\")[3]) !== marker) {\n      loop(name);\n    }\n  };\n  if (first()) {\n    loop(\"ind\\u0435xOf\");\n  } else if (!second()) {\n    loop(\"ind\\u0435xOf\");\n  } else {\n    loop(\"indexOf\");\n  }\n});\nsp();\n";

#[test]
fn a_console_disable_with_a_function_getter_is_removed() {
    let source = format!("{CONTROLLER}{THE_CONSOLE_DISABLE_WITH_A_GETTER}{REST}");
    assert_eq!(safe(&source).trim(), REST);
}

#[test]
fn a_source_probe_written_with_function_expressions_is_removed() {
    let source = format!("{CONTROLLER}{THE_SOURCE_PROBE}{REST}");
    assert_eq!(safe(&source).trim(), REST);
}

#[test]
fn a_getter_that_is_read_as_well_as_called_is_kept() {
    let source = format!(
        "{CONTROLLER}{}{REST}",
        THE_CONSOLE_DISABLE_WITH_A_GETTER.replacen(
            "var that = getGlobal();",
            "var that = getGlobal.call(globalThis);",
            1
        )
    );
    assert_eq!(safe(&source), printed(&source));
}

const THE_DECLARED_FORMAT_CHECK: &str = "var fc = ctl(this, function () {\n  function test() {\n    return !test.constructor(\"return /\\\" + this + \\\"/\")().constructor(\"^([^ ]+( +[^ ]+)+)+[^ ]}\").test(fc);\n  }\n  return test();\n});\nfc();\n";

#[test]
fn a_format_check_written_as_a_declaration_is_removed() {
    let source = format!("{CONTROLLER}{THE_DECLARED_FORMAT_CHECK}{REST}");
    assert_eq!(safe(&source).trim(), REST);
}
