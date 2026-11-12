"use strict";

function _inheritsLoose(t, o) { t.prototype = Object.create(o.prototype), t.prototype.constructor = t, _setPrototypeOf(t, o); }
function _setPrototypeOf(t, e) { return _setPrototypeOf = Object.setPrototypeOf ? Object.setPrototypeOf.bind() : function (t, e) { return t.__proto__ = e, t; }, _setPrototypeOf(t, e); }
function _defineAccessor(e, r, n, t) { var c = { configurable: !0, enumerable: !0 }; return c[e] = t, Object.defineProperty(r, n, c); }
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
  _defineAccessor("get", _proto, "kind", function () {
    return "animal";
  });
  return Animal;
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
  _defineAccessor("get", _proto2, "kind", function () {
    return "dog:" + _Animal.prototype.kind;
  });
  Dog.create = function create(name) {
    var d = _Animal.create.call(this, name);
    d.breed = "mutt";
    return d;
  };
  return Dog;
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
