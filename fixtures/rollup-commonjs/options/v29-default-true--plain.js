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

  var log;
  var hasRequiredLog;

  function requireLog () {
  	if (hasRequiredLog) return log;
  	hasRequiredLog = 1;
  	var count = 0;
  	log = function log(line) {
  	  count += 1;
  	  console.log(count + " " + line);
  	};
  	return log;
  }

  var logExports = requireLog();

  var reassign = {exports: {}};

  var hasRequiredReassign;

  function requireReassign () {
  	if (hasRequiredReassign) return reassign.exports;
  	hasRequiredReassign = 1;
  	requireLog()("reassign eval");
  	reassign.exports = function shout(text) {
  	  return text + "!";
  	};
  	reassign.exports.extra = 7;
  	return reassign.exports;
  }

  var reassignExports = requireReassign();

  var props$1 = {};

  var hasRequiredProps;

  function requireProps () {
  	if (hasRequiredProps) return props$1;
  	hasRequiredProps = 1;
  	requireLog()("props eval");
  	props$1.one = 1;
  	props$1.two = function () {
  	  return 2;
  	};
  	return props$1;
  }

  var propsExports = requireProps();

  var props = /*#__PURE__*/_mergeNamespaces({
    __proto__: null,
    default: propsExports
  }, [propsExports]);

  var cycleA = {};

  var cycleB = {};

  var hasRequiredCycleB;

  function requireCycleB () {
  	if (hasRequiredCycleB) return cycleB;
  	hasRequiredCycleB = 1;
  	const log = requireLog();
  	log("b start");
  	const a = requireCycleA();
  	cycleB.name = "b";
  	cycleB.sawA = String(a.name) + "/" + String(a.fromB);
  	log("b end");
  	return cycleB;
  }

  var hasRequiredCycleA;

  function requireCycleA () {
  	if (hasRequiredCycleA) return cycleA;
  	hasRequiredCycleA = 1;
  	const log = requireLog();
  	log("a start");
  	cycleA.name = "a";
  	const b = requireCycleB();
  	cycleA.fromB = b.name + ":" + b.sawA;
  	log("a end");
  	return cycleA;
  }

  var cycleAExports = requireCycleA();

  logExports("esm start");
  logExports("shout " + reassignExports("y") + " " + reassignExports.extra);
  logExports("props " + Object.keys(props).sort().join(",") + " " + propsExports.one);
  logExports("cycle " + cycleAExports.fromB);

})();
