mod common;

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::{print, PrintStyle};

const DPF: &str = "function dpf(ret) {\n  function debuggerProtection(counter) {\n    if (typeof counter === 'string') {\n      return function (arg) {}['constructor']('while (true) {}')['apply']('counter');\n    } else {\n      debugger;\n    }\n    debuggerProtection(++counter);\n  }\n  try {\n    if (ret) {\n      return debuggerProtection;\n    } else {\n      debuggerProtection(0);\n    }\n  } catch (y) {}\n}\n";

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
    let source =
        format!("{DPF}(function () {{\n  that['setInterval'](dpf, 4000);\n}})();\nconsole.log(1);");
    assert_eq!(run(&alloc, &source).trim(), "console.log(1);");
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
fn a_debugger_nested_one_level_deeper_in_the_alternate_is_still_found() {
    let alloc = Allocator::default();
    let out = run(&alloc, "function dpf(ret) {\n  function inner(counter) {\n    if (typeof counter === 'string') {\n      a();\n    } else {\n      if (counter) {\n        debugger;\n      }\n    }\n    inner(++counter);\n  }\n  try {\n    if (ret) {\n      return inner;\n    } else {\n      inner(0);\n    }\n  } catch (y) {}\n}\nconsole.log(1);");
    assert_eq!(out.trim(), "console.log(1);");
}

#[test]
fn a_constructor_debugger_marker_is_found_through_an_else_if_chain() {
    let alloc = Allocator::default();
    let out = run(&alloc, "function dpf(ret) {\n  function inner(counter) {\n    if (typeof counter === 'string') {\n      a();\n    } else if (counter % 20 === 0) {\n      (function () {\n        return true;\n      })['constructor']('debugger')['call']('action');\n    } else {\n      b();\n    }\n    inner(++counter);\n  }\n  try {\n    if (ret) {\n      return inner;\n    } else {\n      inner(0);\n    }\n  } catch (y) {}\n}\nconsole.log(1);");
    assert_eq!(out.trim(), "console.log(1);");
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

const A_SAME_NAMED_LOCAL_INTERVAL: &str = "function other() {\n  var dpf = 1;\n  (function () {\n    that['setInterval'](dpf, 4000);\n  })();\n  return dpf;\n}\nconsole.log(other());";

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
fn extended_a_string_literal_left_at_the_head_stays_inert() {
    let alloc = Allocator::default();
    let source = format!("{DPF}\"use strict\";\nconsole.log(1);");
    let out =
        common::run_ctors_extended(&alloc, &source, &untwist_deobfuscate::protections_ctors());
    assert_eq!(out.trim(), "(\"use strict\");\nconsole.log(1);");
    assert_eq!(common::directive_count(&out), 0);
}
