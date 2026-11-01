System.register(['path'], (function () {
  'use strict';
  var require$$0;
  return {
    setters: [function (module) {
      require$$0 = module.default;
    }],
    execute: (function () {

      function say(line) {
        console.log(line);
      }
      say("eval say");

      say("eval greet");
      function greet(name) {
        return "hello " + name;
      }
      function shout(name) {
        return name.toUpperCase() + "!";
      }

      say("eval math");
      const add = (a, b) => a + b;
      const mul = (a, b) => a * b;
      function triple(n) {
        return mul(n, 3);
      }

      var math = /*#__PURE__*/Object.freeze({
        __proto__: null,
        add: add,
        mul: mul,
        triple: triple
      });

      say("eval live");
      let counter = 0;
      function bump() {
        counter += 1;
      }

      say("eval reexport");

      var name = "fixture";
      var items = [
      	1,
      	2,
      	3
      ];
      var data = {
      	name: name,
      	items: items
      };

      var data$1 = /*#__PURE__*/Object.freeze({
        __proto__: null,
        default: data,
        items: items,
        name: name
      });

      function getDefaultExportFromCjs (x) {
      	return x && x.__esModule && Object.prototype.hasOwnProperty.call(x, 'default') ? x['default'] : x;
      }

      var cjs$1 = {exports: {}};

      var hasRequiredCjs;

      function requireCjs () {
      	if (hasRequiredCjs) return cjs$1.exports;
      	hasRequiredCjs = 1;
      	(function (module, exports) {
      		const path = require$$0;
      		console.log("eval cjs");
      		exports.join = function (a, b) {
      		  return path.posix.join(a, b);
      		};
      		exports.tag = "cjs-" + 'object'; 
      	} (cjs$1, cjs$1.exports));
      	return cjs$1.exports;
      }

      var cjsExports = requireCjs();
      var cjs = /*@__PURE__*/getDefaultExportFromCjs(cjsExports);

      var cycleA = {};

      var cycleB = {};

      var hasRequiredCycleB;

      function requireCycleB () {
      	if (hasRequiredCycleB) return cycleB;
      	hasRequiredCycleB = 1;
      	console.log("eval cycle-b");
      	const a = requireCycleA();
      	cycleB.seen = "b saw " + a.early + "," + a.late;
      	cycleB.check = function () {
      	  return "b later " + a.late;
      	};
      	return cycleB;
      }

      var hasRequiredCycleA;

      function requireCycleA () {
      	if (hasRequiredCycleA) return cycleA;
      	hasRequiredCycleA = 1;
      	console.log("eval cycle-a start");
      	cycleA.early = "a-early";
      	const b = requireCycleB();
      	cycleA.late = "a-late";
      	cycleA.report = function () {
      	  return b.seen + " " + b.check();
      	};
      	console.log("eval cycle-a end");
      	return cycleA;
      }

      var cycleAExports = requireCycleA();
      var cycle = /*@__PURE__*/getDefaultExportFromCjs(cycleAExports);

      say("eval index");
      say("greet " + greet("ada") + " " + shout("bob"));
      say("math " + Object.keys(math).sort().join(",") + " " + add(2, 3));
      say("live " + counter);
      bump();
      say("live " + counter);
      say("reexport " + shout("x") + " " + triple(4));
      say("json " + data.name + " " + data.items.length);
      say("cjs " + cjs.join("a", "b") + " " + cjs.tag);
      say("cycle " + cycle.report());
      Promise.resolve().then(function () { return data$1; }).then((lazy) => say("json lazy " + (lazy.default || lazy).name));
      Promise.resolve().then(function () { return lazy$1; }).then((lazy) => say("lazy " + lazy.value + " " + lazy.default));

      say("eval lazy");
      const value = 42;
      var lazy = "lazy-default";

      var lazy$1 = /*#__PURE__*/Object.freeze({
        __proto__: null,
        default: lazy,
        value: value
      });

    })
  };
}));
