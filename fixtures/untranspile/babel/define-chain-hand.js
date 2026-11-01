function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
var log = [];
function show(label, o) {
  var parts = Object.getOwnPropertyNames(o).map(function (name) {
    var d = Object.getOwnPropertyDescriptor(o, name);
    var v = typeof d.value === "function" ? "fn:" + d.value.name : JSON.stringify(d.value);
    return name + "=" + v + (d.enumerable ? "e" : "") + (d.writable ? "w" : "") + (d.configurable ? "c" : "");
  });
  console.log(label, parts.join(" "));
}
function run(label, body) {
  try {
    body();
  } catch (error) {
    console.log(label, "threw", error.constructor.name);
  }
}
run("key-to-string", function () {
  var key = { toString: function () { return "k" + log.length; } };
  show("key-to-string", _defineProperty(_defineProperty({}, key, (log.push(1), 2)), "plain", 3));
});
run("reused-base", function () {
  var base = { a: 1 };
  var out = _defineProperty(_defineProperty(base, "b", 2), "c", 3);
  console.log("reused-base", out === base);
  show("reused-base", base);
});
run("temp-read-later", function () {
  var _tmp;
  var out = _defineProperty(_defineProperty(_tmp = {}, "a", 1), "b", 2);
  console.log("temp-read-later", out === _tmp);
  show("temp-read-later", _tmp);
});
run("temp-in-value", function () {
  var _tmp;
  var out = _defineProperty(_tmp = {}, "self", _tmp);
  console.log("temp-in-value", out.self === out);
});
run("temp-read-in-value", function () {
  var _tmp;
  function peek() { return _tmp && Object.keys(_tmp).join("+"); }
  var out = (_tmp = { x: 1 }, _defineProperty(_defineProperty(_tmp, "a", peek()), "b", peek()));
  show("temp-read-in-value", out);
});
run("temp-throws", function () {
  var _tmp;
  function boom() { throw new RangeError("boom"); }
  try {
    _defineProperty(_defineProperty(_tmp = {}, "a", 1), "b", boom());
  } finally {
    console.log("temp-throws", JSON.stringify(_tmp));
  }
});
run("statement-run", function () {
  var _obj;
  function make(v) {
    _obj = { first: v };
    _defineProperty(_defineProperty(_obj, "a", log.length), "b", 2);
    _defineProperty(_obj, "c", 3);
    return _defineProperty(_obj, "d", 4);
  }
  show("statement-run", make(1));
  show("statement-run-again", make(2));
});
run("statement-run-kept", function () {
  var _obj;
  _obj = { first: 0 };
  _defineProperty(_defineProperty(_obj, "a", 1), "b", 2);
  _defineProperty(_obj, "c", 3);
  show("statement-run-kept", _obj);
});
run("getter-base", function () {
  var out = _defineProperty({ get g() { return 1; } }, "g", 2);
  show("getter-base", out);
});
run("proto-base", function () {
  var proto = { inherited: 1 };
  var out = _defineProperty({ __proto__: proto }, "inherited", 2);
  show("proto-base", out);
  console.log("proto-base", Object.getPrototypeOf(out) === proto);
});
run("anonymous-values", function () {
  show("anonymous-values", _defineProperty(_defineProperty(_defineProperty({}, "f", function () {}), "g", function () { return 1; }), "c", class {}));
});
run("symbol-key", function () {
  var sym = Symbol("s");
  var out = _defineProperty({}, sym, 1);
  console.log("symbol-key", out[sym], Object.getOwnPropertySymbols(out).length);
});
run("numeric-keys", function () {
  show("numeric-keys", _defineProperty(_defineProperty(_defineProperty({}, 2, "two"), -1, "minus"), 1.5, "half"));
});
console.log(log.join(","));
