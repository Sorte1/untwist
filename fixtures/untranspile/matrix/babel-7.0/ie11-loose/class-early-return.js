"use strict";

function _assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function _inheritsLoose(subClass, superClass) { subClass.prototype = Object.create(superClass.prototype); subClass.prototype.constructor = subClass; subClass.__proto__ = superClass; }

var Resource = function Resource(name) {
  this.name = name;
};

var Pooled =
/*#__PURE__*/
function (_Resource) {
  _inheritsLoose(Pooled, _Resource);

  function Pooled(name, size) {
    var _this;

    if (size === void 0) {
      size = 4;
    }

    _this = _Resource.call(this, name) || this;
    _this.size = size;

    if (size > 8) {
      _this.capped = true;
      return _assertThisInitialized(_this);
    }

    _this.items = [];
    return _this;
  }

  return Pooled;
}(Resource);

var first = new Pooled("a"),
    second = new Pooled("b", 16);
console.log(first.items.length, second.capped, second.items, first instanceof Resource);
