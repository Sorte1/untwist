"use strict";

function _templateObject4() {
  var data = _taggedTemplateLiteral(["A", "\n"], ["\\u{41}", "\\n"]);

  _templateObject4 = function _templateObject4() {
    return data;
  };

  return data;
}

function _templateObject3() {
  var data = _taggedTemplateLiteral(["x"]);

  _templateObject3 = function _templateObject3() {
    return data;
  };

  return data;
}

function _templateObject2() {
  var data = _taggedTemplateLiteral(["\n", "\t"], ["\\n", "\\t"]);

  _templateObject2 = function _templateObject2() {
    return data;
  };

  return data;
}

function _templateObject() {
  var data = _taggedTemplateLiteral(["a", "b", "c"]);

  _templateObject = function _templateObject() {
    return data;
  };

  return data;
}

function _taggedTemplateLiteral(strings, raw) { if (!raw) { raw = strings.slice(0); } return Object.freeze(Object.defineProperties(strings, { raw: { value: Object.freeze(raw) } })); }

var name = "world";
var n = 3;
console.log("hello ".concat(name), "".concat(n, " + ").concat(n, " = ").concat(n + n), "nested ".concat("inner ".concat(name)));
console.log("multi\nline", "".concat({
  toString: function toString() {
    return "obj";
  }
}), "".concat([1, 2]));

function tag(strings) {
  for (var _len = arguments.length, values = new Array(_len > 1 ? _len - 1 : 0), _key = 1; _key < _len; _key++) {
    values[_key - 1] = arguments[_key];
  }

  return strings.raw.join("|") + "#" + strings.join("|") + "#" + values.join(",");
}

console.log(tag(_templateObject(), 1, 2), tag(_templateObject2(), "x"));

function same(s) {
  return s;
}

var first = function first() {
  return same(_templateObject3());
};

console.log(first() === first(), Object.isFrozen(first()), String.raw(_templateObject4(), 1));
var sym = {
  valueOf: function valueOf() {
    return 1;
  },
  toString: function toString() {
    return "str";
  }
};
console.log("".concat(sym), "" + sym);
