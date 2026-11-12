"use strict";

function _inheritsLoose(t, o) { t.prototype = Object.create(o.prototype), t.prototype.constructor = t, _setPrototypeOf(t, o); }
function _setPrototypeOf(t, e) { return _setPrototypeOf = Object.setPrototypeOf ? Object.setPrototypeOf.bind() : function (t, e) { return t.__proto__ = e, t; }, _setPrototypeOf(t, e); }
var Shape = function Shape(kind) {
  this.kind = kind;
};
var Box = /*#__PURE__*/function (_Shape) {
  function Box(width, height, label) {
    var _this;
    if (width === void 0) {
      width = 1;
    }
    if (height === void 0) {
      height = width * 2;
    }
    _this = _Shape.call(this, "box") || this;
    _this.width = width;
    _this.height = height;
    _this.label = label;
    return _this;
  }
  _inheritsLoose(Box, _Shape);
  var _proto = Box.prototype;
  _proto.area = function area(scale) {
    if (scale === void 0) {
      scale = 1;
    }
    return this.width * this.height * scale;
  };
  return Box;
}(Shape);
var Tagged = /*#__PURE__*/function (_Box) {
  function Tagged(tag) {
    var _this2;
    if (tag === void 0) {
      tag = "t";
    }
    for (var _len = arguments.length, rest = new Array(_len > 1 ? _len - 1 : 0), _key = 1; _key < _len; _key++) {
      rest[_key - 1] = arguments[_key];
    }
    _this2 = _Box.call.apply(_Box, [this].concat(rest)) || this;
    _this2.tag = tag;
    return _this2;
  }
  _inheritsLoose(Tagged, _Box);
  return Tagged;
}(Box);
function later(step) {
  if (step === void 0) {
    step = 1;
  }
  for (var _len2 = arguments.length, more = new Array(_len2 > 1 ? _len2 - 1 : 0), _key2 = 1; _key2 < _len2; _key2++) {
    more[_key2 - 1] = arguments[_key2];
  }
  var run = function run() {
    return step + more.length;
  };
  return run();
}
console.log(JSON.stringify(new Box()), JSON.stringify(new Box(3)), JSON.stringify(new Box(3, 4, "x")));
console.log(new Box(2).area(), new Box(2).area(3), Box.length, Box.prototype.area.length);
console.log(JSON.stringify(new Tagged()), JSON.stringify(new Tagged("q", 5)), Tagged.length);
console.log(later(), later(2, 0, 0), later.length);
