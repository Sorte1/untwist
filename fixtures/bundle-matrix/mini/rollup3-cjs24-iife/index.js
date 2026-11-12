(function () {
  'use strict';

  console.log("eval cjs");
  var twice = function (n) {
    return n * 2;
  };

  console.log("eval index " + twice(21));
  Promise.resolve().then(function () { return lazy; }).then(lazy => console.log("lazy " + lazy.value));

  console.log("eval lazy");
  const value = 42;

  var lazy = /*#__PURE__*/Object.freeze({
    __proto__: null,
    value: value
  });

})();
