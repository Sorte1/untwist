(function () {
  'use strict';

  function greet() {
    var name = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : "anon";
    var greeting = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : "hi " + name;
    return greeting + "!";
  }
  console.log(greet(), greet("bo"), greet(undefined, "yo"), greet(null));
  function collect(first) {
    for (var _len = arguments.length, rest = new Array(_len > 1 ? _len - 1 : 0), _key = 1; _key < _len; _key++) {
      rest[_key - 1] = arguments[_key];
    }
    return first + ":" + rest.join(",") + ":" + rest.length;
  }
  console.log(collect(1), collect(1, 2, 3));
  function counted(a) {
    var b = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : 2;
    var c = arguments.length > 2 ? arguments[2] : undefined;
    return arguments.length + ":" + a + b + c;
  }
  console.log(counted(1), counted(1, undefined, 3), counted.length);
  var arrow = function arrow() {
    var x = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : 1;
    return x + (arguments.length <= 1 ? 0 : arguments.length - 1);
  };
  console.log(arrow(), arrow(5, 6, 7), arrow.length);
  var evals = 0;
  function lazy() {
    var v = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : ++evals;
    return v;
  }
  lazy(1);
  lazy();
  lazy();
  console.log(evals);
  function scoped(a) {
    var f = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : function () {
      return a;
    };
    return function (a) {
      return f();
    }();
  }
  console.log(scoped(1));
  function withTemps(opts) {
    var _opts$width;
    var size = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : 10;
    var label = arguments.length > 2 && arguments[2] !== undefined ? arguments[2] : null;
    var width = (_opts$width = opts === null || opts === void 0 ? void 0 : opts.width) !== null && _opts$width !== void 0 ? _opts$width : size;
    return width + ":" + label;
  }
  console.log(withTemps({
    width: 3
  }), withTemps(null, 4, "x"), withTemps.length);
  function inlined(a) {
    var flag = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : false;
    var mode = arguments.length > 2 && arguments[2] !== undefined ? arguments[2] : "m";
    return [a, flag, mode].join("/");
  }
  console.log(inlined(1), inlined(1, true), inlined(1, undefined, "z"));

})();
