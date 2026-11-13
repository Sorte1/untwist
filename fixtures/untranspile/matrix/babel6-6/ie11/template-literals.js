"use strict";

var _templateObject = _taggedTemplateLiteral(["a", "b", "c"], ["a", "b", "c"]),
    _templateObject2 = _taggedTemplateLiteral(["\n", "\t"], ["\\n", "\\t"]),
    _templateObject3 = _taggedTemplateLiteral(["x"], ["x"]),
    _templateObject4 = _taggedTemplateLiteral(["A", "\n"], ["\\u{41}", "\\n"]);

function _taggedTemplateLiteral(strings, raw) { return Object.freeze(Object.defineProperties(strings, { raw: { value: Object.freeze(raw) } })); }

var name = "world";
var n = 3;
console.log("hello " + name, n + " + " + n + " = " + (n + n), "nested " + ("inner " + name));
console.log("multi\nline", "" + {
  toString: function toString() {
    return "obj";
  }
}, "" + [1, 2]);
function tag(strings) {
  for (var _len = arguments.length, values = Array(_len > 1 ? _len - 1 : 0), _key = 1; _key < _len; _key++) {
    values[_key - 1] = arguments[_key];
  }

  return strings.raw.join("|") + "#" + strings.join("|") + "#" + values.join(",");
}
console.log(tag(_templateObject, 1, 2), tag(_templateObject2, "x"));
function same(s) {
  return s;
}
var first = function first() {
  return same(_templateObject3);
};
console.log(first() === first(), Object.isFrozen(first()), String.raw(_templateObject4, 1));
var sym = {
  valueOf: function valueOf() {
    return 1;
  },
  toString: function toString() {
    return "str";
  }
};
console.log("" + sym, "" + sym);
