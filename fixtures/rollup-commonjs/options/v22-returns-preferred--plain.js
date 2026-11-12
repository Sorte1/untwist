var app = (function () {
  'use strict';

  var requireEsm = {};

  var count = 0;
  var log$1 = function log(line) {
    count += 1;
    console.log(count + " " + line);
  };

  log$1("esm-lib eval");
  var require$$1 = "d";

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
  const lib = require$$1;
  log("lib " + typeof lib + " " + (lib));
  function late() {
    return requireLazyInner().value;
  }
  log("late " + late());

  return requireEsm;

})();
