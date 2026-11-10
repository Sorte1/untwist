mod common;

use common::cpu_or_wall;
use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;

const SMALL: usize = 10_000;
const LARGE: usize = 40_000;
const GROWTH_BOUND: u32 = 8;

fn bundle_with_helpers(count: usize) -> String {
    let helpers: String = (0..count)
        .map(|index| format!("  __webpack_require__.a{index} = function () {{ return {index}; }};\n"))
        .collect();
    format!(
        "(function (modules) {{
  var installedModules = {{}};
  function __webpack_require__(moduleId) {{
    if (installedModules[moduleId]) return installedModules[moduleId].exports;
    var module = installedModules[moduleId] = {{ i: moduleId, l: false, exports: {{}} }};
    modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
    module.l = true;
    return module.exports;
  }}
{helpers}  return __webpack_require__(__webpack_require__.s = 0);
}})([
  function (module, exports, __webpack_require__) {{ module.exports = __webpack_require__.a1(); }}
]);
"
    )
}

fn unpack_time(count: usize) -> std::time::Duration {
    let source = bundle_with_helpers(count);
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let (unpacked, spent) = cpu_or_wall(|| {
        let mut programs = vec![parsed.program];
        untwist_unpack::unpack_inputs(&alloc, &mut programs, &[], Default::default())
            .0
            .map(|bundle| bundle.modules.len())
    });
    assert_eq!(unpacked, Some(2), "{count} helpers");
    spent
}

#[test]
fn helper_routing_grows_linearly_with_the_number_of_helpers() {
    let small = unpack_time(SMALL);
    let large = unpack_time(LARGE);
    let ratio = (LARGE / SMALL) as u32;
    assert!(
        large < small * ratio * GROWTH_BOUND / 4,
        "{SMALL} helpers took {small:?}, {LARGE} took {large:?}"
    );
}
