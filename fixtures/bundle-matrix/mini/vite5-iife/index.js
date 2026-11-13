(function() {
  "use strict";
  console.log("eval cjs");
  var twice = function(n) {
    return n * 2;
  };
  console.log("eval index " + twice(21));
  Promise.resolve().then(() => lazy).then((lazy2) => console.log("lazy " + lazy2.value));
  console.log("eval lazy");
  const value = 42;
  const lazy = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
    __proto__: null,
    value
  }, Symbol.toStringTag, { value: "Module" }));
})();
