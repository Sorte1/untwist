(function () {
  "use strict";
  const k = Object.keys({ constructor: 1 })[0];
  const F = ({ __proto__: Object })[k];
  F("return this")().parseInt = () => null;
  const p = parseInt("12");
  const s = "abc".toUpperCase();
  console.log(typeof p, typeof s);
})();
