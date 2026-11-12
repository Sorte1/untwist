"use strict";

var _createClass = function () { function defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } } return function (Constructor, protoProps, staticProps) { if (protoProps) defineProperties(Constructor.prototype, protoProps); if (staticProps) defineProperties(Constructor, staticProps); return Constructor; }; }();

function _possibleConstructorReturn(self, call) { if (!self) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return call && (typeof call === "object" || typeof call === "function") ? call : self; }

function _inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function, not " + typeof superClass); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, enumerable: false, writable: true, configurable: true } }); if (superClass) Object.setPrototypeOf ? Object.setPrototypeOf(subClass, superClass) : subClass.__proto__ = superClass; }

function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

var seq = 0;

var Counter = function Counter(step) {
  var _this = this;

  _classCallCheck(this, Counter);

  this.count = 0;
  this.step = 1;
  this.id = ++seq;
  this.label = "c" + this.id;
  this["dyn" + seq] = "dynamic";

  this.inc = function () {
    _this.count += _this.step;
    return _this;
  };

  if (step) this.step = step;
  Counter.instances++;
};

Counter.instances = 0;
Counter.created = Counter.instances + 1;

var c = new Counter(2);
c.inc().inc();
console.log(c.count, c.label, c.dyn1, Counter.instances, Counter.created);
var inc = c.inc;

inc();
console.log(c.count, Object.keys(c).join(","));

var Base = function () {
  function Base() {
    _classCallCheck(this, Base);

    this.setup();
  }

  _createClass(Base, [{
    key: "setup",
    value: function setup() {}
  }]);

  return Base;
}();

var Derived = function (_Base) {
  _inherits(Derived, _Base);

  function Derived() {
    var _ref;

    var _temp, _this2, _ret;

    _classCallCheck(this, Derived);

    for (var _len = arguments.length, args = Array(_len), _key = 0; _key < _len; _key++) {
      args[_key] = arguments[_key];
    }

    return _ret = (_temp = (_this2 = _possibleConstructorReturn(this, (_ref = Derived.__proto__ || Object.getPrototypeOf(Derived)).call.apply(_ref, [this].concat(args))), _this2), _this2.value = 5, _temp), _possibleConstructorReturn(_this2, _ret);
  }

  _createClass(Derived, [{
    key: "setup",
    value: function setup() {
      this.seen = this.value;
    }
  }]);

  return Derived;
}(Base);

var dd = new Derived();
console.log(dd.seen, dd.value);

var Shadow = function () {
  function Shadow() {
    _classCallCheck(this, Shadow);
  }

  _createClass(Shadow, [{
    key: "x",
    set: function set(v) {
      this.setterRan = true;
    }
  }]);

  return Shadow;
}();

var Defines = function (_Shadow) {
  _inherits(Defines, _Shadow);

  function Defines() {
    var _ref2;

    var _temp2, _this3, _ret2;

    _classCallCheck(this, Defines);

    for (var _len2 = arguments.length, args = Array(_len2), _key2 = 0; _key2 < _len2; _key2++) {
      args[_key2] = arguments[_key2];
    }

    return _ret2 = (_temp2 = (_this3 = _possibleConstructorReturn(this, (_ref2 = Defines.__proto__ || Object.getPrototypeOf(Defines)).call.apply(_ref2, [this].concat(args))), _this3), _this3.x = 1, _temp2), _possibleConstructorReturn(_this3, _ret2);
  }

  return Defines;
}(Shadow);

var df = new Defines();
console.log(df.x, df.setterRan === true);
