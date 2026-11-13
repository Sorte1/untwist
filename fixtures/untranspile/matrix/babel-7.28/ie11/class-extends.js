"use strict";

function _callSuper(t, o, e) { return o = _getPrototypeOf(o), _possibleConstructorReturn(t, _isNativeReflectConstruct() ? Reflect.construct(o, e || [], _getPrototypeOf(t).constructor) : o.apply(t, e)); }
function _possibleConstructorReturn(t, e) { if (e && ("object" == _typeof(e) || "function" == typeof e)) return e; if (void 0 !== e) throw new TypeError("Derived constructors may only return object or undefined"); return _assertThisInitialized(t); }
function _assertThisInitialized(e) { if (void 0 === e) throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); return e; }
function _isNativeReflectConstruct() { try { var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); } catch (t) {} return (_isNativeReflectConstruct = function _isNativeReflectConstruct() { return !!t; })(); }
function _superPropGet(t, o, e, r) { var p = _get(_getPrototypeOf(1 & r ? t.prototype : t), o, e); return 2 & r && "function" == typeof p ? function (t) { return p.apply(e, t); } : p; }
function _get() { return _get = "undefined" != typeof Reflect && Reflect.get ? Reflect.get.bind() : function (e, t, r) { var p = _superPropBase(e, t); if (p) { var n = Object.getOwnPropertyDescriptor(p, t); return n.get ? n.get.call(arguments.length < 3 ? e : r) : n.value; } }, _get.apply(null, arguments); }
function _superPropBase(t, o) { for (; !{}.hasOwnProperty.call(t, o) && null !== (t = _getPrototypeOf(t));); return t; }
function _getPrototypeOf(t) { return _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf.bind() : function (t) { return t.__proto__ || Object.getPrototypeOf(t); }, _getPrototypeOf(t); }
function _inherits(t, e) { if ("function" != typeof e && null !== e) throw new TypeError("Super expression must either be null or a function"); t.prototype = Object.create(e && e.prototype, { constructor: { value: t, writable: !0, configurable: !0 } }), Object.defineProperty(t, "prototype", { writable: !1 }), e && _setPrototypeOf(t, e); }
function _setPrototypeOf(t, e) { return _setPrototypeOf = Object.setPrototypeOf ? Object.setPrototypeOf.bind() : function (t, e) { return t.__proto__ = e, t; }, _setPrototypeOf(t, e); }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
var Animal = /*#__PURE__*/function () {
  function Animal(name) {
    _classCallCheck(this, Animal);
    this.name = name;
  }
  return _createClass(Animal, [{
    key: "speak",
    value: function speak() {
      return this.name + " makes a sound";
    }
  }, {
    key: "kind",
    get: function get() {
      return "animal";
    }
  }], [{
    key: "create",
    value: function create(name) {
      return new this(name);
    }
  }]);
}();
var Dog = /*#__PURE__*/function (_Animal) {
  function Dog(name, breed) {
    var _this;
    _classCallCheck(this, Dog);
    _this = _callSuper(this, Dog, [name]);
    _this.breed = breed;
    return _this;
  }
  _inherits(Dog, _Animal);
  return _createClass(Dog, [{
    key: "speak",
    value: function speak() {
      return _superPropGet(Dog, "speak", this, 3)([]) + " (woof)";
    }
  }, {
    key: "kind",
    get: function get() {
      return "dog:" + _superPropGet(Dog, "kind", this, 1);
    }
  }], [{
    key: "create",
    value: function create(name) {
      var d = _superPropGet(Dog, "create", this, 2)([name]);
      d.breed = "mutt";
      return d;
    }
  }]);
}(Animal);
var Puppy = /*#__PURE__*/function (_Dog) {
  function Puppy() {
    _classCallCheck(this, Puppy);
    return _callSuper(this, Puppy, arguments);
  }
  _inherits(Puppy, _Dog);
  return _createClass(Puppy);
}(Dog);
var d = new Dog("Rex", "lab");
console.log(d.speak(), d.kind, d instanceof Animal, d.breed);
var m = Dog.create("Max");
console.log(m instanceof Dog, m.breed, m.speak());
var pup = new Puppy("Bit", "pug");
console.log(pup.speak(), Object.getPrototypeOf(Puppy) === Dog, Puppy.name);
function Legacy(v) {
  this.v = v;
}
Legacy.prototype.get = function () {
  return this.v;
};
var Modern = /*#__PURE__*/function (_Legacy) {
  function Modern() {
    _classCallCheck(this, Modern);
    return _callSuper(this, Modern, arguments);
  }
  _inherits(Modern, _Legacy);
  return _createClass(Modern, [{
    key: "get",
    value: function get() {
      return _superPropGet(Modern, "get", this, 3)([]) * 2;
    }
  }]);
}(Legacy);
console.log(new Modern(21).get());
