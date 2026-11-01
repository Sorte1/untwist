System.register([], function (_export, _context) {
  "use strict";
  var value;
  return {
    setters: [],
    execute: function () {
      console.log("eval entry");
      _export("value", value = 3);
      console.log("value", value);
    }
  };
});
