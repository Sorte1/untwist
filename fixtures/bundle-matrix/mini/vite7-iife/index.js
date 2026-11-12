(function() {
  "use strict";
  var cjs = {};
  var hasRequiredCjs;
  function requireCjs() {
    if (hasRequiredCjs) return cjs;
    hasRequiredCjs = 1;
    console.log("eval cjs");
    cjs.twice = function(n) {
      return n * 2;
    };
    return cjs;
  }
  var cjsExports = requireCjs();
  console.log("eval index " + cjsExports.twice(21));
  Promise.resolve().then(() => lazy).then((lazy2) => console.log("lazy " + lazy2.value));
  console.log("eval lazy");
  const value = 42;
  const lazy = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
    __proto__: null,
    value
  }, Symbol.toStringTag, { value: "Module" }));
})();
