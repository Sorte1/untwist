var q = 1; function zz() { return 2; }
require = function (k) { return globalThis[k]; }; console.log(q + zz(), typeof require("a"), typeof require("b"));
