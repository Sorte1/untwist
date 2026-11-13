function _inherits(t, r) { if ("function" != typeof r && null !== r) throw new TypeError("Super expression must either be null or a function"); t.prototype = Object.create(r && r.prototype, { constructor: { value: t, writable: !0, configurable: !0 } }), Object.defineProperty(t, "prototype", { writable: !1 }), r && _setPrototypeOf(t, r); }
function _setPrototypeOf(t, e) { return _setPrototypeOf = Object.setPrototypeOf ? Object.setPrototypeOf.bind() : function (t, e) { return t.__proto__ = e, t; }, _setPrototypeOf(t, e); }
function _callSuper(e, c, p) { return c = _getPrototypeOf(c), _possibleConstructorReturn(e, _isNativeReflectConstruct() ? Reflect.construct(c, p || [], _getPrototypeOf(e).constructor) : c.apply(e, p)); }
function _possibleConstructorReturn(t, e) { if (e && ("object" == _typeof(e) || "function" == typeof e)) return e; if (void 0 !== e) throw new TypeError("Derived constructors may only return object or undefined"); return _assertThisInitialized(t); }
function _assertThisInitialized(e) { if (void 0 === e) throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); return e; }
function _isNativeReflectConstruct() { try { var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); } catch (t) {} return (_isNativeReflectConstruct = function _isNativeReflectConstruct() { return !!t; })(); }
function _superPropGet(r, e, p, f) { var n = _get(_getPrototypeOf(1 & f ? r.prototype : r), e, p); return 2 & f && "function" == typeof n ? function (t) { return n.apply(p, t); } : n; }
function _get() { return _get = "undefined" != typeof Reflect && Reflect.get ? Reflect.get.bind() : function (t, r, n) { var l = _superPropBase(t, r); if (l) { var f = Object.getOwnPropertyDescriptor(l, r); return f.get ? f.get.call(arguments.length < 3 ? t : n) : f.value; } }, _get.apply(null, arguments); }
function _superPropBase(o, t) { for (; !{}.hasOwnProperty.call(o, t) && null !== (o = _getPrototypeOf(o));); return o; }
function _getPrototypeOf(t) { return _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf.bind() : function (t) { return t.__proto__ || Object.getPrototypeOf(t); }, _getPrototypeOf(t); }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(r, t) { for (var o = 0; o < t.length; o++) { var i = t[o]; i.enumerable = i.enumerable || !1, i.configurable = !0, "value" in i && (i.writable = !0), Object.defineProperty(r, _toPropertyKey(i.key), i); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(r) { var o = _toPrimitive(r, "string"); return "symbol" == _typeof(o) ? o : o + ""; }
function _toPrimitive(t, e) { if ("object" != _typeof(t) || !t) return t; var r; if ("undefined" != typeof Symbol && void 0 !== (r = t[Symbol.toPrimitive])) { var i = r.call(t, e || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === e ? String : Number)(t); }
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
