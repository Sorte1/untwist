function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
var fns = [];
var _loop = function _loop(i) {
  fns.push(function () {
    return i;
  });
};
for (var i = 0; i < 3; i++) {
  _loop(i);
}
console.log(fns.map(function (f) {
  return f();
}).join(","));
var fns2 = [];
var _loop2 = function _loop2(k) {
  fns2.push(function () {
    return k;
  });
};
for (var k in {
  a: 1,
  b: 2
}) {
  _loop2(k);
}
console.log(fns2.map(function (f) {
  return f();
}).join(","));
{
  var shadow = "inner";
  console.log(shadow);
}
var obj = {
  name: "o",
  arrow: function arrow() {
    var _this = this;
    return [1].map(function () {
      return _this.name;
    });
  },
  plain: function plain() {
    return [1].map(function () {
      return _typeof(this);
    });
  }
};
console.log(obj.arrow()[0], obj.plain()[0]);
function argsArrow() {
  var _arguments = arguments;
  var a = function a() {
    return _arguments.length;
  };
  return a(1, 2, 3);
}
console.log(argsArrow(7));
var tpl = function tpl(x) {
  return "v=".concat(x, " ").concat(_typeof(x));
};
console.log(tpl(1), tpl.name, tpl.length);
var def = function def(a) {
  var b = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : a + 1;
  return [a, b, arguments.length <= 2 ? 0 : arguments.length - 2];
};
console.log(def(1).join(","), def.length);
try {
  var tdz = function tdz() {
    return later;
  };
  tdz();
  var later = 1;
} catch (e) {
  console.log(e.constructor.name);
}
var hoisted = _typeof(laterFn);
function laterFn() {}
console.log(hoisted);
console.log(_typeof(function () {}.prototype), _typeof(function () {}.prototype));
var closures = [];
var j = 0;
var _loop3 = function _loop3() {
  var captured = j;
  closures.push(function () {
    return captured;
  });
  j++;
};
while (j < 2) {
  _loop3();
}
console.log(closures.map(function (f) {
  return f();
}).join(","));
var short = {
  a: a,
  b: function b() {
    return "b";
  }
};
function a() {
  return "a";
}
console.log(short.a(), short.b(), _typeof(short.b.prototype));
