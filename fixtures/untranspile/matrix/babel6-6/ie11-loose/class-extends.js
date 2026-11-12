"use strict";

var _createClass = function () { function defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } } return function (Constructor, protoProps, staticProps) { if (protoProps) defineProperties(Constructor.prototype, protoProps); if (staticProps) defineProperties(Constructor, staticProps); return Constructor; }; }();

function _possibleConstructorReturn(self, call) { if (!self) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return call && (typeof call === "object" || typeof call === "function") ? call : self; }

function _inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function, not " + typeof superClass); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, enumerable: false, writable: true, configurable: true } }); if (superClass) Object.setPrototypeOf ? Object.setPrototypeOf(subClass, superClass) : subClass.__proto__ = superClass; }

function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

var Animal = function () {
  function Animal(name) {
    _classCallCheck(this, Animal);

    this.name = name;
  }

  Animal.prototype.speak = function speak() {
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

var Dog = function (_Animal) {
  _inherits(Dog, _Animal);

  function Dog(name, breed) {
    _classCallCheck(this, Dog);

    var _this = _possibleConstructorReturn(this, _Animal.call(this, name));

    _this.breed = breed;
    return _this;
  }

  Dog.prototype.speak = function speak() {
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

var Puppy = function (_Dog) {
  _inherits(Puppy, _Dog);

  function Puppy() {
    _classCallCheck(this, Puppy);

    return _possibleConstructorReturn(this, _Dog.apply(this, arguments));
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

var Modern = function (_Legacy) {
  _inherits(Modern, _Legacy);

  function Modern() {
    _classCallCheck(this, Modern);

    return _possibleConstructorReturn(this, _Legacy.apply(this, arguments));
  }

  Modern.prototype.get = function get() {
    return _Legacy.prototype.get.call(this) * 2;
  };

  return Modern;
}(Legacy);

console.log(new Modern(21).get());
