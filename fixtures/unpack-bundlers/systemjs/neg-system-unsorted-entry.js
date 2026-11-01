System.register([], function (_export) {
  "use strict";
  return {
    execute: function () {
      console.log("eval entry");
      _export("zeta", 1);
      _export("alpha", 2);
    }
  };
});
