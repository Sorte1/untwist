"use strict";

function _inheritsLoose(subClass, superClass) { subClass.prototype = Object.create(superClass.prototype); subClass.prototype.constructor = subClass; subClass.__proto__ = superClass; }

function _defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function _createClass(Constructor, protoProps, staticProps) { if (protoProps) _defineProperties(Constructor.prototype, protoProps); if (staticProps) _defineProperties(Constructor, staticProps); return Constructor; }

var Animal =
/*#__PURE__*/
function () {
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

  _createClass(Animal, [{
    key: "kind",
    get: function get() {
      return "animal";
    }
  }]);

  return Animal;
}();

var Dog =
/*#__PURE__*/
function (_Animal) {
  _inheritsLoose(Dog, _Animal);

  function Dog(name, breed) {
    var _this;

    _this = _Animal.call(this, name) || this;
    _this.breed = breed;
    return _this;
  }

  var _proto2 = Dog.prototype;

  _proto2.speak = function speak() {
    return _Animal.prototype.speak.call(this) + " (woof)";
  };

  Dog.create = function create(name) {
    var d = _Animal.create.call(this, name);

    d.breed = "mutt";
    return d;
  };

  _createClass(Dog, [{
    key: "kind",
    get: function get() {
      return "dog:" + _Animal.prototype.kind;
    }
  }]);

  return Dog;
}(Animal);

var Puppy =
/*#__PURE__*/
function (_Dog) {
  _inheritsLoose(Puppy, _Dog);

  function Puppy() {
    return _Dog.apply(this, arguments) || this;
  }

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

var Modern =
/*#__PURE__*/
function (_Legacy) {
  _inheritsLoose(Modern, _Legacy);

  function Modern() {
    return _Legacy.apply(this, arguments) || this;
  }

  var _proto3 = Modern.prototype;

  _proto3.get = function get() {
    return _Legacy.prototype.get.call(this) * 2;
  };

  return Modern;
}(Legacy);

console.log(new Modern(21).get());
