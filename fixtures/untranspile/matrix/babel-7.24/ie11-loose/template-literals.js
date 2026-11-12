"use strict";

var _templateObject, _templateObject2, _templateObject3, _templateObject4;
function _taggedTemplateLiteralLoose(e, t) { return t || (t = e.slice(0)), e.raw = t, e; }
var name = "world";
var n = 3;
console.log("hello " + name, n + " + " + n + " = " + (n + n), "nested " + ("inner " + name));
console.log("multi\nline", "" + {
  toString: function toString() {
    return "obj";
  }
}, "" + [1, 2]);
function tag(strings) {
  for (var _len = arguments.length, values = new Array(_len > 1 ? _len - 1 : 0), _key = 1; _key < _len; _key++) {
    values[_key - 1] = arguments[_key];
  }
  return strings.raw.join("|") + "#" + strings.join("|") + "#" + values.join(",");
}
console.log(tag(_templateObject || (_templateObject = _taggedTemplateLiteralLoose(["a", "b", "c"])), 1, 2), tag(_templateObject2 || (_templateObject2 = _taggedTemplateLiteralLoose(["\n", "\t"], ["\\n", "\\t"])), "x"));
function same(s) {
  return s;
}
var first = function first() {
  return same(_templateObject3 || (_templateObject3 = _taggedTemplateLiteralLoose(["x"])));
};
console.log(first() === first(), Object.isFrozen(first()), String.raw(_templateObject4 || (_templateObject4 = _taggedTemplateLiteralLoose(["A", "\n"], ["\\u{41}", "\\n"])), 1));
var sym = {
  valueOf: function valueOf() {
    return 1;
  },
  toString: function toString() {
    return "str";
  }
};
console.log("" + sym, "" + sym);
