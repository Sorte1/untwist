"use strict";

function _possibleConstructorReturn(self, call) { if (!self) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return call && (typeof call === "object" || typeof call === "function") ? call : self; }

function _inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function, not " + typeof superClass); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, enumerable: false, writable: true, configurable: true } }); if (superClass) Object.setPrototypeOf ? Object.setPrototypeOf(subClass, superClass) : subClass.__proto__ = superClass; }

function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

var Resource = function Resource(name) {
  _classCallCheck(this, Resource);

  this.name = name;
};

var Pooled = function (_Resource) {
  _inherits(Pooled, _Resource);

  function Pooled(name) {
    var size = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : 4;

    _classCallCheck(this, Pooled);

    var _this = _possibleConstructorReturn(this, _Resource.call(this, name));

    _this.size = size;
    if (size > 8) {
      _this.capped = true;
      return _possibleConstructorReturn(_this);
    }
    _this.items = [];
    return _this;
  }

  return Pooled;
}(Resource);

var first = new Pooled("a"),
    second = new Pooled("b", 16);

console.log(first.items.length, second.capped, second.items, first instanceof Resource);
