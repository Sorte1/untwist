(function () {
  'use strict';

  var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");
  var _typeof2 = _interopRequireDefault(require("@babel/runtime/helpers/typeof"));
  var _createClass2 = _interopRequireDefault(require("@babel/runtime/helpers/createClass"));
  var _classCallCheck2 = _interopRequireDefault(require("@babel/runtime/helpers/classCallCheck"));
  var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));
  var _Config;
  var Config = /*#__PURE__*/(0, _createClass2.default)(function Config() {
    (0, _classCallCheck2.default)(this, Config);
  });
  _Config = Config;
  (0, _defineProperty2.default)(Config, "defaults", {
    mode: "fast"
  });
  (0, _defineProperty2.default)(Config, "ready", void 0);
  (function () {
    _Config.ready = Object.keys(_Config.defaults).length > 0;
    _Config.loaded = "yes";
  })();
  (function () {
    var local = 3;
    _Config.triple = local * 3;
  })();
  console.log(Config.ready, Config.loaded, Config.triple, typeof local === "undefined" ? "undefined" : (0, _typeof2.default)(local));

})();
