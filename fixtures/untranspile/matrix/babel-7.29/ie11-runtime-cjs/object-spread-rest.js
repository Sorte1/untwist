(function () {
  'use strict';

  var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");
  var _typeof = require("@babel/runtime/helpers/typeof");
  var _objectWithoutProperties2 = _interopRequireDefault(require("@babel/runtime/helpers/objectWithoutProperties"));
  var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));
  var _excluded = ["secret"];
  function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
  function _toPrimitive(t, e) { if ("object" != _typeof(t) || !t) return t; var r; if ("undefined" != typeof Symbol && void 0 !== (r = t[Symbol.toPrimitive])) { var i = r.call(t, e); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return (String )(t); }
  function ownKeys(e, r) { var t = Object.keys(e); if (Object.getOwnPropertySymbols) { var o = Object.getOwnPropertySymbols(e); r && (o = o.filter(function (r) { return Object.getOwnPropertyDescriptor(e, r).enumerable; })), t.push.apply(t, o); } return t; }
  function _objectSpread(e) { for (var r = 1; r < arguments.length; r++) { var t = null != arguments[r] ? arguments[r] : {}; r % 2 ? ownKeys(Object(t), true).forEach(function (r) { (0, _defineProperty2.default)(e, r, t[r]); }) : Object.getOwnPropertyDescriptors ? Object.defineProperties(e, Object.getOwnPropertyDescriptors(t)) : ownKeys(Object(t)).forEach(function (r) { Object.defineProperty(e, r, Object.getOwnPropertyDescriptor(t, r)); }); } return e; }
  var base = {
    a: 1,
    b: 2
  };
  var extra = {
    c: 3
  };
  var merged = _objectSpread(_objectSpread(_objectSpread({}, base), extra), {}, {
    d: 4
  });
  var overridden = _objectSpread(_objectSpread({}, base), {}, {
    a: 9
  });
  console.log(JSON.stringify(merged), JSON.stringify(overridden));
  var a = merged.a,
    others = (0, _objectWithoutProperties2.default)(merged, ["a"]);
  console.log(a, JSON.stringify(others));
  function without(_ref) {
    _ref.secret;
      var pub = (0, _objectWithoutProperties2.default)(_ref, _excluded);
    return pub;
  }
  console.log(JSON.stringify(without({
    secret: 1,
    open: 2
  })));
  var withGetter = _objectSpread({}, {
    get g() {
      return "got";
    }
  });
  console.log(Object.getOwnPropertyDescriptor(withGetter, "g").value);
  var sym = Symbol("s");
  var symObj = _objectSpread({}, (0, _defineProperty2.default)((0, _defineProperty2.default)({}, sym, 1), "plain", 2));
  console.log(symObj[sym], symObj.plain);
  var picked = symObj[sym],
    restSym = (0, _objectWithoutProperties2.default)(symObj, [sym].map(_toPropertyKey));
  console.log(picked, Object.keys(restSym).join(","));
  console.log(JSON.stringify(_objectSpread(_objectSpread(_objectSpread(_objectSpread({}, null), undefined), "hi"), [5])));
  var order = [];
  var tracked = {
    get x() {
      order.push("x");
      return 1;
    },
    get y() {
      order.push("y");
      return 2;
    }
  };
  var copy = _objectSpread({}, tracked);
  console.log(order.join(","), copy.x + copy.y);

})();
