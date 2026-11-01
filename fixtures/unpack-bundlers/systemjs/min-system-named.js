System.register("app", ["lib", "helpers", "path"], function (_export, _context) {
  "use strict";
  var greet, counter, bump, Shape, helpers, path;
  return {
    setters: [function (m) {
      greet = m.greet;
      counter = m.counter;
      bump = m.bump;
      Shape = m.Shape;
      _export("total", m.total);
    }, function (m) {
      helpers = m;
    }, function (m) {
      path = m.default;
    }],
    execute: function () {
      console.log("eval app");
      var double = helpers.double;
      console.log("greet", greet("ada"), "double", double(4), "join", path.posix.join("a", "b"));
      console.log("live", counter);
      bump();
      console.log("live", counter);
      console.log("area", new Shape(3).area(), helpers.tag);
      _export("version", 2);
      _context.import("lazy").then(function (ns) {
        console.log("lazy", ns.value, ns.default, Object.keys(ns).join(","));
      });
    }
  };
});
System.register("lib", [], function (_export) {
  "use strict";
  var total;
  return {
    execute: function () {
      _export({ bump: bump, greet: greet });
      console.log("eval lib");
      let counter = _export("counter", 0);
      function bump() {
        _export("counter", counter += 1);
      }
      function greet(name) {
        return "hi " + name;
      }
      class Shape {
        constructor(side) {
          this.side = side;
        }
        area() {
          return this.side * this.side;
        }
      }
      _export("Shape", Shape);
      _export("total", total = 10);
    }
  };
});
System.register("helpers", [], function (_export) {
  "use strict";
  return {
    execute: function () {
      console.log("eval helpers");
      _export("double", function (n) {
        return n * 2;
      });
      _export("tag", "helpers-" + typeof Map);
    }
  };
});
System.register("lazy", [], function (_export) {
  "use strict";
  var value;
  return {
    execute: function () {
      console.log("eval lazy");
      _export("default", "lazy-default");
      _export("value", value = 42);
    }
  };
});
