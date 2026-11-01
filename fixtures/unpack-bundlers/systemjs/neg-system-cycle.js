System.register("a", ["b"], function (_export) {
  "use strict";
  var b;
  return {
    setters: [function (m) {
      b = m;
    }],
    execute: function () {
      console.log("eval a", b.late);
      _export("early", "a-early");
    }
  };
});
System.register("b", ["a"], function (_export) {
  "use strict";
  var a;
  return {
    setters: [function (m) {
      a = m;
    }],
    execute: function () {
      console.log("eval b", a === undefined ? "unbound" : typeof a.early);
      _export("late", "b-late");
    }
  };
});
