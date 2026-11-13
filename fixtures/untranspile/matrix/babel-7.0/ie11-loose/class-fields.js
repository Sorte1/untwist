"use strict";

function _defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function _createClass(Constructor, protoProps, staticProps) { if (protoProps) _defineProperties(Constructor.prototype, protoProps); if (staticProps) _defineProperties(Constructor, staticProps); return Constructor; }

function _inheritsLoose(subClass, superClass) { subClass.prototype = Object.create(superClass.prototype); subClass.prototype.constructor = subClass; subClass.__proto__ = superClass; }

var seq = 0;

var _ref = "dyn" + seq;

var Counter = function Counter(step) {
  var _this = this;

  this.count = 0;
  this.step = 1;
  this.id = ++seq;
  this.label = "c" + this.id;
  this[_ref] = "dynamic";

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

var Base =
/*#__PURE__*/
function () {
  function Base() {
    this.setup();
  }

  var _proto = Base.prototype;

  _proto.setup = function setup() {};

  return Base;
}();

var Derived =
/*#__PURE__*/
function (_Base) {
  _inheritsLoose(Derived, _Base);

  function Derived() {
    var _this2;

    for (var _len = arguments.length, args = new Array(_len), _key = 0; _key < _len; _key++) {
      args[_key] = arguments[_key];
    }

    _this2 = _Base.call.apply(_Base, [this].concat(args)) || this;
    _this2.value = 5;
    return _this2;
  }

  var _proto2 = Derived.prototype;

  _proto2.setup = function setup() {
    this.seen = this.value;
  };

  return Derived;
}(Base);

var dd = new Derived();
console.log(dd.seen, dd.value);

var Shadow =
/*#__PURE__*/
function () {
  function Shadow() {}

  _createClass(Shadow, [{
    key: "x",
    set: function set(v) {
      this.setterRan = true;
    }
  }]);

  return Shadow;
}();

var Defines =
/*#__PURE__*/
function (_Shadow) {
  _inheritsLoose(Defines, _Shadow);

  function Defines() {
    var _this3;

    for (var _len2 = arguments.length, args = new Array(_len2), _key2 = 0; _key2 < _len2; _key2++) {
      args[_key2] = arguments[_key2];
    }

    _this3 = _Shadow.call.apply(_Shadow, [this].concat(args)) || this;
    _this3.x = 1;
    return _this3;
  }

  return Defines;
}(Shadow);

var df = new Defines();
console.log(df.x, df.setterRan === true);
