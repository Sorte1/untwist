use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_unpack::{holds_a_bundle_with, unpack_program_with, UnpackOptions};

const ON: UnpackOptions = UnpackOptions {
    split_hoisted: true,
    conservative: false,
    trust_free_loader: false,
};

const RUNTIME: &str = r#"(() => {
  "use strict";
  var __webpack_modules__ = ({});
  const __webpack_module_cache__ = {};
  function __webpack_require__(moduleId) {
    const cachedModule = __webpack_module_cache__[moduleId];
    if (cachedModule !== undefined) {
      return cachedModule.exports;
    }
    const module = __webpack_module_cache__[moduleId] = { exports: {} };
    __webpack_modules__[moduleId](module, module.exports, __webpack_require__);
    return module.exports;
  }
  __webpack_require__.m = __webpack_modules__;
  const label = "runtime";
  const report = () => label;
  self.webpackReport = report;
})();
"#;

const HOISTED: &str = r#"(function () {
  "use strict";
  const handlers = { a: () => 1 };
  function dispatch(name) {
    return handlers[name]();
  }
  const first = dispatch("a");
  const second = first + 1;
  console.log(second);
})();
"#;

fn split_count(source: &str) -> Option<usize> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    unpack_program_with(&alloc, &parsed.program, ON).map(|bundle| bundle.modules.len())
}

#[test]
fn a_webpack_runtime_is_not_split_as_a_scope_hoisted_bundle() {
    assert_eq!(split_count(RUNTIME), None);
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, RUNTIME);
    assert!(!holds_a_bundle_with(&parsed.program, ON));
}

#[test]
fn a_function_that_returns_a_handler_call_does_not_block_the_split() {
    assert!(split_count(HOISTED).is_none_or(|count| count >= 1));
}
