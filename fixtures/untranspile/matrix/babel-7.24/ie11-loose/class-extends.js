"use strict";

function _inheritsLoose(t, o) { t.prototype = Object.create(o.prototype), t.prototype.constructor = t, _setPrototypeOf(t, o); }
function _setPrototypeOf(t, e) { return _setPrototypeOf = Object.setPrototypeOf ? Object.setPrototypeOf.bind() : function (t, e) { return t.__proto__ = e, t; }, _setPrototypeOf(t, e); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == typeof i ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != typeof t || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != typeof i) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
var Animal = /*#__PURE__*/function () {
  function Animal(name) {
    this.name = name;
  }
  var _proto = Animal.prototype;
  _proto.speak = function speak() {
    return this.name + " makes a sound";
  };
  Animal.create = function create(name) {
    return new this(name);
  };
  return _createClass(Animal, [{
    key: "kind",
    get: function get() {
      return "animal";
    }
  }]);
}();
var Dog = /*#__PURE__*/function (_Animal) {
  function Dog(name, breed) {
    var _this;
    _this = _Animal.call(this, name) || this;
    _this.breed = breed;
    return _this;
  }
  _inheritsLoose(Dog, _Animal);
  var _proto2 = Dog.prototype;
  _proto2.speak = function speak() {
    return _Animal.prototype.speak.call(this) + " (woof)";
  };
  Dog.create = function create(name) {
    var d = _Animal.create.call(this, name);
    d.breed = "mutt";
    return d;
  };
  return _createClass(Dog, [{
    key: "kind",
    get: function get() {
      return "dog:" + _Animal.prototype.kind;
    }
  }]);
}(Animal);
var Puppy = /*#__PURE__*/function (_Dog) {
  function Puppy() {
    return _Dog.apply(this, arguments) || this;
  }
  _inheritsLoose(Puppy, _Dog);
  return Puppy;
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
    return _Legacy.apply(this, arguments) || this;
  }
  _inheritsLoose(Modern, _Legacy);
  var _proto3 = Modern.prototype;
  _proto3.get = function get() {
    return _Legacy.prototype.get.call(this) * 2;
  };
  return Modern;
}(Legacy);
console.log(new Modern(21).get());
