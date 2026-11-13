var app = (function () {
  'use strict';

  var requireEsm = {};

  var count = 0;
  var log$1 = function log(line) {
    count += 1;
    console.log(count + " " + line);
  };

  log$1("esm-lib eval");
  const named = "n";
  var esmLib = "d";

  var esmLib$1 = /*#__PURE__*/Object.freeze({
    __proto__: null,
    named: named,
    'default': esmLib
  });

  var lazyInner = {};

  var hasRequiredLazyInner;

  function requireLazyInner () {
  	if (hasRequiredLazyInner) return lazyInner;
  	hasRequiredLazyInner = 1;
  	log$1("lazy-inner eval");
  	lazyInner.value = 42;
  	return lazyInner;
  }

  const log = log$1;
  const lib = esmLib$1;
  log("lib " + typeof lib + " " + (typeof lib === "object" ? Object.keys(lib).sort().join(",") : lib));
  function late() {
    return requireLazyInner().value;
  }
  log("late " + late());

  return requireEsm;

})();
