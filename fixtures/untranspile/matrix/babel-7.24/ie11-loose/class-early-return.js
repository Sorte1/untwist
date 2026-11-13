"use strict";

function _assertThisInitialized(e) { if (void 0 === e) throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); return e; }
function _inheritsLoose(t, o) { t.prototype = Object.create(o.prototype), t.prototype.constructor = t, _setPrototypeOf(t, o); }
function _setPrototypeOf(t, e) { return _setPrototypeOf = Object.setPrototypeOf ? Object.setPrototypeOf.bind() : function (t, e) { return t.__proto__ = e, t; }, _setPrototypeOf(t, e); }
var Resource = function Resource(name) {
  this.name = name;
};
var Pooled = /*#__PURE__*/function (_Resource) {
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
  _inheritsLoose(Pooled, _Resource);
  return Pooled;
}(Resource);
var first = new Pooled("a"),
  second = new Pooled("b", 16);
console.log(first.items.length, second.capped, second.items, first instanceof Resource);
