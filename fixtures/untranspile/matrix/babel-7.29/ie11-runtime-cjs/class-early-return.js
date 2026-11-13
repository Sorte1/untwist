(function () {
  'use strict';

  var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");
  var _possibleConstructorReturn2 = _interopRequireDefault(require("@babel/runtime/helpers/possibleConstructorReturn"));
  var _getPrototypeOf2 = _interopRequireDefault(require("@babel/runtime/helpers/getPrototypeOf"));
  var _inherits2 = _interopRequireDefault(require("@babel/runtime/helpers/inherits"));
  var _createClass2 = _interopRequireDefault(require("@babel/runtime/helpers/createClass"));
  var _classCallCheck2 = _interopRequireDefault(require("@babel/runtime/helpers/classCallCheck"));
  function _callSuper(t, o, e) { return o = (0, _getPrototypeOf2.default)(o), (0, _possibleConstructorReturn2.default)(t, _isNativeReflectConstruct() ? Reflect.construct(o, e || [], (0, _getPrototypeOf2.default)(t).constructor) : o.apply(t, e)); }
  function _isNativeReflectConstruct() { try { var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); } catch (t) {} return (_isNativeReflectConstruct = function _isNativeReflectConstruct() { return !!t; })(); }
  var Resource = /*#__PURE__*/(0, _createClass2.default)(function Resource(name) {
    (0, _classCallCheck2.default)(this, Resource);
    this.name = name;
  });
  var Pooled = /*#__PURE__*/function (_Resource) {
    function Pooled(name) {
      var _this;
      var size = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : 4;
      (0, _classCallCheck2.default)(this, Pooled);
      _this = _callSuper(this, Pooled, [name]);
      _this.size = size;
      if (size > 8) {
        _this.capped = true;
        return (0, _possibleConstructorReturn2.default)(_this);
      }
      _this.items = [];
      return _this;
    }
    (0, _inherits2.default)(Pooled, _Resource);
    return (0, _createClass2.default)(Pooled);
  }(Resource);
  var first = new Pooled("a"),
    second = new Pooled("b", 16);
  console.log(first.items.length, second.capped, second.items, first instanceof Resource);

})();
