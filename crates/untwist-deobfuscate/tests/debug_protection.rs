mod common;

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::pass::PassOptions;
use untwist_core::print::{print, PrintStyle};

const DPF: &str = "function dpf(ret) {\n  function debuggerProtection(counter) {\n    if (typeof counter === \"string\") {\n      return function (arg) {}.constructor(\"while (true) {}\").apply(\"counter\");\n    } else if ((\"\" + counter / counter).length !== 1 || counter % 20 === 0) {\n      (function () {\n        return true;\n      }).constructor(\"debugger\").call(\"action\");\n    } else {\n      (function () {\n        return false;\n      }).constructor(\"debugger\").apply(\"stateObject\");\n    }\n    debuggerProtection(++counter);\n  }\n  try {\n    if (ret) {\n      return debuggerProtection;\n    } else {\n      debuggerProtection(0);\n    }\n  } catch (y) {}\n}\n";

const INTERVAL: &str = "(function () {\n  let that;\n  try {\n    const getGlobal = Function(\"return (function() {}.constructor(\\\"return this\\\")( ));\");\n    that = getGlobal();\n  } catch (e) {\n    that = window;\n  }\n  that.setInterval(dpf, 4000);\n})();\n";

fn run<'a>(alloc: &'a Allocator, source: &'a str) -> String {
    common::run_ctors(alloc, source, &untwist_deobfuscate::protections_ctors()).0
}

fn run_with_notes<'a>(alloc: &'a Allocator, source: &'a str) -> (String, Vec<String>) {
    common::run_ctors(alloc, source, &untwist_deobfuscate::protections_ctors())
}

fn printed(source: &str) -> String {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    print(&parsed.program, PrintStyle::Pretty)
}

#[test]
fn the_debug_protection_function_is_removed() {
    let alloc = Allocator::default();
    let source = format!("{DPF}console.log(1);");
    assert_eq!(run(&alloc, &source).trim(), "console.log(1);");
}

#[test]
fn the_var_function_form_is_removed_too() {
    let alloc = Allocator::default();
    let source = format!(
        "{}console.log(1);",
        DPF.replacen("function dpf(ret)", "var dpf = function (ret)", 1)
            .replacen("\n}\n", "\n};\n", 1)
    );
    assert_eq!(run(&alloc, &source).trim(), "console.log(1);");
}

#[test]
fn the_set_interval_iife_is_removed() {
    let alloc = Allocator::default();
    let source = format!("{DPF}{INTERVAL}console.log(1);");
    assert_eq!(run(&alloc, &source).trim(), "console.log(1);");
}

#[test]
fn the_default_lane_removes_the_protection_and_its_interval() {
    let alloc = Allocator::default();
    let source = format!("{DPF}{INTERVAL}console.log(1);");
    let (out, _) = untwist_testing::transform_report(
        &alloc,
        &source,
        &untwist_deobfuscate::protections_ctors(),
        PassOptions::default(),
    );
    assert_eq!(out.trim(), "console.log(1);");
}

#[test]
fn an_interval_on_an_intact_global_is_removed_as_a_statement() {
    for timer in [
        "setInterval(dpf, 4000);",
        "window.setInterval(dpf, 4000);",
        "globalThis.setInterval(dpf, 4000);",
    ] {
        let alloc = Allocator::default();
        let source = format!("{DPF}{timer}\nconsole.log(1);");
        assert_eq!(run(&alloc, &source).trim(), "console.log(1);", "{timer}");
    }
}

#[test]
fn an_iife_that_does_other_work_loses_only_its_interval_call() {
    let alloc = Allocator::default();
    let source = format!(
        "{DPF}(function () {{\n  console.log(\"real work\");\n  globalThis.started = true;\n  globalThis.setInterval(dpf, 4000);\n}})();\nconsole.log(globalThis.started);"
    );
    let out = run(&alloc, &source);
    assert_eq!(
        out.trim(),
        "(function () {\n  console.log(\"real work\");\n  globalThis.started = true;\n})();\nconsole.log(globalThis.started);"
    );
}

const A_FUNCTION_THAT_ONLY_RESEMBLES_THE_PROTECTION: &str = "function dp(ret) {\n  function inner(counter) {\n    if (counter > 2) {\n      console.log(\"payload runs every tick\");\n      return;\n    } else {\n      debugger;\n    }\n    inner(++counter);\n  }\n  try {\n    if (ret) {\n      return inner;\n    } else {\n      inner(0);\n    }\n  } catch (e) {}\n}\n(function () {\n  console.log(\"real work in the same iife\");\n  globalThis.started = true;\n  globalThis.setInterval(dp, 4000);\n})();\nconsole.log(globalThis.started);";

#[test]
fn a_function_that_only_resembles_the_protection_keeps_its_interval_and_its_iife() {
    for safe in [true, false] {
        let alloc = Allocator::default();
        let (out, _) = untwist_testing::transform_report(
            &alloc,
            A_FUNCTION_THAT_ONLY_RESEMBLES_THE_PROTECTION,
            &untwist_deobfuscate::protections_ctors(),
            PassOptions {
                unsafe_transforms: !safe,
                ..PassOptions::default()
            },
        );
        assert_eq!(
            out,
            printed(A_FUNCTION_THAT_ONLY_RESEMBLES_THE_PROTECTION),
            "safe={safe}"
        );
    }
}

#[test]
fn an_interval_on_an_object_the_program_built_refuses_the_removal() {
    let alloc = Allocator::default();
    let source = format!(
        "{DPF}(function () {{\n  var that = {{ setInterval: function (f) {{ console.log(\"payload\"); }} }};\n  that.setInterval(dpf, 4000);\n}})();"
    );
    let (out, notes) = run_with_notes(&alloc, &source);
    assert_eq!(out, printed(&source));
    assert!(
        notes
            .iter()
            .any(|note| note == untwist_deobfuscate::notes::DEBUG_PROTECTION_REFERENCE_KEPT),
        "{notes:?}"
    );
}

#[test]
fn an_interval_on_a_timer_the_program_replaced_refuses_the_removal() {
    for patch in [
        "setInterval = function (f) { console.log(\"payload\"); };\nsetInterval(dpf, 4000);",
        "function setInterval(f) { console.log(\"payload\"); }\nsetInterval(dpf, 4000);",
        "window = { setInterval: function (f) { console.log(\"payload\"); } };\nwindow.setInterval(dpf, 4000);",
    ] {
        let alloc = Allocator::default();
        let source = format!("{DPF}{patch}");
        let out = run(&alloc, &source);
        assert_eq!(out, printed(&source), "{patch}");
    }
}

#[test]
fn a_protection_function_with_one_more_statement_is_kept() {
    let alloc = Allocator::default();
    let source = format!(
        "{}{INTERVAL}",
        DPF.replacen("  try {\n", "  console.log(\"payload\");\n  try {\n", 1)
    );
    let out = run(&alloc, &source);
    assert_eq!(out, printed(&source));
}

#[test]
fn a_function_without_the_debugger_marker_is_kept() {
    let alloc = Allocator::default();
    let source = "function dpf(ret) {\n  function inner(counter) {\n    if (ret) {\n      a();\n    } else {\n      b();\n    }\n    inner(++counter);\n  }\n  try {\n    if (ret) {\n      return inner;\n    } else {\n      inner(0);\n    }\n  } catch (y) {}\n}\nconsole.log(1);";
    let out = run(&alloc, source);
    assert!(out.contains("function dpf"), "{out}");
    assert_eq!(out, printed(source));
}

#[test]
fn a_try_body_that_is_not_the_protection_shape_is_kept() {
    let alloc = Allocator::default();
    let source = "function dpf(ret) {\n  function inner(counter) {\n    if (typeof counter === 'string') {\n      a();\n    } else {\n      debugger;\n    }\n    inner(++counter);\n  }\n  try {\n    log(ret);\n  } catch (y) {}\n}\nconsole.log(1);";
    let out = run(&alloc, source);
    assert!(out.contains("function dpf"), "{out}");
    assert_eq!(out, printed(source));
}

#[test]
fn a_body_with_a_debugger_statement_but_other_code_is_kept() {
    for source in [
        "function dpf(ret) {\n  function inner(counter) {\n    if (typeof counter === 'string') {\n      a();\n    } else {\n      if (counter) {\n        debugger;\n      }\n    }\n    inner(++counter);\n  }\n  try {\n    if (ret) {\n      return inner;\n    } else {\n      inner(0);\n    }\n  } catch (y) {}\n}\nconsole.log(1);",
        "function dpf(ret) {\n  function inner(counter) {\n    if (typeof counter === 'string') {\n      a();\n    } else if (counter % 20 === 0) {\n      (function () {\n        return true;\n      })['constructor']('debugger')['call']('action');\n    } else {\n      b();\n    }\n    inner(++counter);\n  }\n  try {\n    if (ret) {\n      return inner;\n    } else {\n      inner(0);\n    }\n  } catch (y) {}\n}\nconsole.log(1);",
    ] {
        let alloc = Allocator::default();
        let out = run(&alloc, source);
        assert_eq!(out, printed(source));
    }
}

#[test]
fn under_conservative_a_string_literal_left_at_the_head_becomes_a_directive_again() {
    let alloc = Allocator::default();
    let source = format!("{DPF}\"use strict\";\nconsole.log(1);");
    let out = run(&alloc, &source);
    assert_eq!(out.trim(), "\"use strict\";\nconsole.log(1);");
    assert!(!out.contains("(\"use strict\")"), "{out}");
    assert_eq!(common::directive_count(&out), 1);
}

#[test]
fn a_string_literal_that_is_not_at_the_head_stays_an_expression() {
    let alloc = Allocator::default();
    let source = format!("console.log(0);\n{DPF}\"use strict\";\nconsole.log(1);");
    let out = run(&alloc, &source);
    assert!(out.contains("console.log(0)"), "{out}");
    assert_eq!(common::directive_count(&out), 0);
}

const A_SAME_NAMED_LOCAL_INTERVAL: &str = "function other() {\n  var dpf = 1;\n  (function () {\n    that.setInterval(dpf, 4000);\n  })();\n  return dpf;\n}\nconsole.log(other());";

#[test]
fn a_same_named_local_keeps_its_own_interval_iife() {
    let alloc = Allocator::default();
    let source = format!("{DPF}{A_SAME_NAMED_LOCAL_INTERVAL}");
    let out = run(&alloc, &source);
    assert_eq!(out, printed(A_SAME_NAMED_LOCAL_INTERVAL));
}

#[test]
fn a_reference_the_template_cannot_account_for_refuses_the_removal() {
    let alloc = Allocator::default();
    let source = format!("{DPF}console.log(typeof dpf);");
    let (out, notes) = run_with_notes(&alloc, &source);
    assert_eq!(out, printed(&source));
    assert!(
        notes
            .iter()
            .any(|note| note == untwist_deobfuscate::notes::DEBUG_PROTECTION_REFERENCE_KEPT),
        "{notes:?}"
    );
}

#[test]
fn the_default_lane_keeps_a_string_literal_left_at_the_head_inert() {
    let alloc = Allocator::default();
    let source = format!("{DPF}\"use strict\";\nconsole.log(1);");
    let out =
        common::run_ctors_default_lane(&alloc, &source, &untwist_deobfuscate::protections_ctors());
    assert_eq!(out.trim(), "(\"use strict\");\nconsole.log(1);");
    assert_eq!(common::directive_count(&out), 0);
}

#[test]
fn under_unsafe_a_string_literal_left_at_the_head_becomes_the_directive() {
    let alloc = Allocator::default();
    let source = format!("{DPF}\"use strict\";\nconsole.log(1);");
    let out =
        common::run_ctors_extended(&alloc, &source, &untwist_deobfuscate::protections_ctors());
    assert_eq!(common::directive_count(&out), 1);
}

const INTERVAL_WITH_A_GETTER: &str = "(function () {\n  var getGlobal = function () {\n    var that;\n    try {\n      that = Function(\"return (function() {}.constructor(\\\"return this\\\")( ));\")();\n    } catch (e) {\n      that = window;\n    }\n    return that;\n  };\n  var that = getGlobal();\n  that.setInterval(dpf, 4000);\n})();\n";

#[test]
fn an_interval_behind_a_function_getter_is_removed() {
    let alloc = Allocator::default();
    let source = format!("{DPF}{INTERVAL_WITH_A_GETTER}console.log(1);");
    let (out, _) = untwist_testing::transform_report(
        &alloc,
        &source,
        &untwist_deobfuscate::protections_ctors(),
        PassOptions::default(),
    );
    assert_eq!(out.trim(), "console.log(1);");
}
