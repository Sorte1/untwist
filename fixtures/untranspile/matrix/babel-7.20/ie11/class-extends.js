"use strict";

function _get() { if (typeof Reflect !== "undefined" && Reflect.get) { _get = Reflect.get.bind(); } else { _get = function _get(target, property, receiver) { var base = _superPropBase(target, property); if (!base) return; var desc = Object.getOwnPropertyDescriptor(base, property); if (desc.get) { return desc.get.call(arguments.length < 3 ? target : receiver); } return desc.value; }; } return _get.apply(this, arguments); }
function _superPropBase(object, property) { while (!Object.prototype.hasOwnProperty.call(object, property)) { object = _getPrototypeOf(object); if (object === null) break; } return object; }
function _inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); Object.defineProperty(subClass, "prototype", { writable: false }); if (superClass) _setPrototypeOf(subClass, superClass); }
function _setPrototypeOf(o, p) { _setPrototypeOf = Object.setPrototypeOf ? Object.setPrototypeOf.bind() : function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return _setPrototypeOf(o, p); }
function _createSuper(Derived) { var hasNativeReflectConstruct = _isNativeReflectConstruct(); return function _createSuperInternal() { var Super = _getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = _getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return _possibleConstructorReturn(this, result); }; }
function _possibleConstructorReturn(self, call) { if (call && (_typeof(call) === "object" || typeof call === "function")) { return call; } else if (call !== void 0) { throw new TypeError("Derived constructors may only return object or undefined"); } return _assertThisInitialized(self); }
function _assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }
function _isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); return true; } catch (e) { return false; } }
function _getPrototypeOf(o) { _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf.bind() : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return _getPrototypeOf(o); }
function _typeof(obj) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (obj) { return typeof obj; } : function (obj) { return obj && "function" == typeof Symbol && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }, _typeof(obj); }
function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }
function _defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, _toPropertyKey(descriptor.key), descriptor); } }
function _createClass(Constructor, protoProps, staticProps) { if (protoProps) _defineProperties(Constructor.prototype, protoProps); if (staticProps) _defineProperties(Constructor, staticProps); Object.defineProperty(Constructor, "prototype", { writable: false }); return Constructor; }
function _toPropertyKey(arg) { var key = _toPrimitive(arg, "string"); return _typeof(key) === "symbol" ? key : String(key); }
function _toPrimitive(input, hint) { if (_typeof(input) !== "object" || input === null) return input; var prim = input[Symbol.toPrimitive]; if (prim !== undefined) { var res = prim.call(input, hint || "default"); if (_typeof(res) !== "object") return res; throw new TypeError("@@toPrimitive must return a primitive value."); } return (hint === "string" ? String : Number)(input); }
var Animal = /*#__PURE__*/function () {
  function Animal(name) {
    _classCallCheck(this, Animal);
    this.name = name;
  }
  _createClass(Animal, [{
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
  return Animal;
}();
var Dog = /*#__PURE__*/function (_Animal) {
  _inherits(Dog, _Animal);
  var _super = _createSuper(Dog);
  function Dog(name, breed) {
    var _this;
    _classCallCheck(this, Dog);
    _this = _super.call(this, name);
    _this.breed = breed;
    return _this;
  }
  _createClass(Dog, [{
    key: "speak",
    value: function speak() {
      return _get(_getPrototypeOf(Dog.prototype), "speak", this).call(this) + " (woof)";
    }
  }, {
    key: "kind",
    get: function get() {
      return "dog:" + _get(_getPrototypeOf(Dog.prototype), "kind", this);
    }
  }], [{
    key: "create",
    value: function create(name) {
      var d = _get(_getPrototypeOf(Dog), "create", this).call(this, name);
      d.breed = "mutt";
      return d;
    }
  }]);
  return Dog;
}(Animal);
var Puppy = /*#__PURE__*/function (_Dog) {
  _inherits(Puppy, _Dog);
  var _super2 = _createSuper(Puppy);
  function Puppy() {
    _classCallCheck(this, Puppy);
    return _super2.apply(this, arguments);
  }
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
  _inherits(Modern, _Legacy);
  var _super3 = _createSuper(Modern);
  function Modern() {
    _classCallCheck(this, Modern);
    return _super3.apply(this, arguments);
  }
  _createClass(Modern, [{
    key: "get",
    value: function get() {
      return _get(_getPrototypeOf(Modern.prototype), "get", this).call(this) * 2;
    }
  }]);
  return Modern;
}(Legacy);
console.log(new Modern(21).get());
