(function () {
  'use strict';

  var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");
  var _possibleConstructorReturn2 = _interopRequireDefault(require("@babel/runtime/helpers/possibleConstructorReturn"));
  var _getPrototypeOf2 = _interopRequireDefault(require("@babel/runtime/helpers/getPrototypeOf"));
  var _get2 = _interopRequireDefault(require("@babel/runtime/helpers/get"));
  var _inherits2 = _interopRequireDefault(require("@babel/runtime/helpers/inherits"));
  var _classCallCheck2 = _interopRequireDefault(require("@babel/runtime/helpers/classCallCheck"));
  var _createClass2 = _interopRequireDefault(require("@babel/runtime/helpers/createClass"));
  function _callSuper(t, o, e) { return o = (0, _getPrototypeOf2.default)(o), (0, _possibleConstructorReturn2.default)(t, _isNativeReflectConstruct() ? Reflect.construct(o, e || [], (0, _getPrototypeOf2.default)(t).constructor) : o.apply(t, e)); }
  function _isNativeReflectConstruct() { try { var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); } catch (t) {} return (_isNativeReflectConstruct = function _isNativeReflectConstruct() { return !!t; })(); }
  function _superPropGet(t, o, e, r) { var p = (0, _get2.default)((0, _getPrototypeOf2.default)(1 & r ? t.prototype : t), o, e); return 2 & r && "function" == typeof p ? function (t) { return p.apply(e, t); } : p; }
  var Animal = /*#__PURE__*/function () {
    function Animal(name) {
      (0, _classCallCheck2.default)(this, Animal);
      this.name = name;
    }
    return (0, _createClass2.default)(Animal, [{
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
      (0, _classCallCheck2.default)(this, Dog);
      _this = _callSuper(this, Dog, [name]);
      _this.breed = breed;
      return _this;
    }
    (0, _inherits2.default)(Dog, _Animal);
    return (0, _createClass2.default)(Dog, [{
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
      (0, _classCallCheck2.default)(this, Puppy);
      return _callSuper(this, Puppy, arguments);
    }
    (0, _inherits2.default)(Puppy, _Dog);
    return (0, _createClass2.default)(Puppy);
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
      (0, _classCallCheck2.default)(this, Modern);
      return _callSuper(this, Modern, arguments);
    }
    (0, _inherits2.default)(Modern, _Legacy);
    return (0, _createClass2.default)(Modern, [{
      key: "get",
      value: function get() {
        return _superPropGet(Modern, "get", this, 3)([]) * 2;
      }
    }]);
  }(Legacy);
  console.log(new Modern(21).get());

})();
