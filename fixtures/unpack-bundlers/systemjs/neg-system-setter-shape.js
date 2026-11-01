System.register("app", ["lib"], function (_export) {
  "use strict";
  var value;
  return {
    setters: [function (m) {
      console.log("setter", m.value);
      value = m.value;
    }],
    execute: function () {
      console.log("eval app", value);
    }
  };
});
System.register("lib", [], function (_export) {
  "use strict";
  return {
    execute: function () {
      console.log("eval lib");
      _export("value", 1);
    }
  };
});
