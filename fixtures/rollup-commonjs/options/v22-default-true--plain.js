(function () {
  'use strict';

  function _mergeNamespaces(n, m) {
    m.forEach(function (e) {
      e && typeof e !== 'string' && !Array.isArray(e) && Object.keys(e).forEach(function (k) {
        if (k !== 'default' && !(k in n)) {
          var d = Object.getOwnPropertyDescriptor(e, k);
          Object.defineProperty(n, k, d.get ? d : {
            enumerable: true,
            get: function () { return e[k]; }
          });
        }
      });
    });
    return Object.freeze(n);
  }

  var count = 0;
  var log = function log(line) {
    count += 1;
    console.log(count + " " + line);
  };

  var reassign = {exports: {}};

  log("reassign eval");
  reassign.exports = function shout(text) {
    return text + "!";
  };
  var extra = reassign.exports.extra = 7;

  var props$1 = {};

  log("props eval");
  var one = props$1.one = 1;
  var two = props$1.two = function () {
    return 2;
  };

  var props = /*#__PURE__*/_mergeNamespaces({
    __proto__: null,
    one: one,
    two: two,
    'default': props$1
  }, [props$1]);

  var cycleA = {};

  var cycleB = {};

  var hasRequiredCycleB;

  function requireCycleB () {
  	if (hasRequiredCycleB) return cycleB;
  	hasRequiredCycleB = 1;
  	const log$1 = log;
  	log$1("b start");
  	const a = requireCycleA();
  	cycleB.name = "b";
  	cycleB.sawA = String(a.name) + "/" + String(a.fromB);
  	log$1("b end");
  	return cycleB;
  }

  var hasRequiredCycleA;

  function requireCycleA () {
  	if (hasRequiredCycleA) return cycleA;
  	hasRequiredCycleA = 1;
  	const log$1 = log;
  	log$1("a start");
  	cycleA.name = "a";
  	const b = requireCycleB();
  	cycleA.fromB = b.name + ":" + b.sawA;
  	log$1("a end");
  	return cycleA;
  }

  var cycleAExports = requireCycleA();

  log("esm start");
  log("shout " + reassign.exports("y") + " " + extra);
  log("props " + Object.keys(props).sort().join(",") + " " + one);
  log("cycle " + cycleAExports.fromB);

})();
