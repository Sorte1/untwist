(function () { "use strict";
var value;
(function () { "use strict";
  var hasRequiredA, a;
  function requireA() { if (hasRequiredA) return a; hasRequiredA = 1; a = function () { return value; }; return a; }
  globalThis.peek = requireA();
  console.log(peek());
})();
value = 5;
console.log(peek(), value);
})();
