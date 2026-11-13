"use strict";

function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function _possibleConstructorReturn(self, call) { if (!self) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return call && (typeof call === "object" || typeof call === "function") ? call : self; }

function _inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function, not " + typeof superClass); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, enumerable: false, writable: true, configurable: true } }); if (superClass) Object.setPrototypeOf ? Object.setPrototypeOf(subClass, superClass) : subClass.__proto__ = superClass; }

var AppError = function (_Error) {
  _inherits(AppError, _Error);

  function AppError(message, code) {
    _classCallCheck(this, AppError);

    var _this = _possibleConstructorReturn(this, _Error.call(this, message));

    _this.name = "AppError";
    _this.code = code;
    return _this;
  }

  AppError.prototype.describe = function describe() {
    return this.name + "[" + this.code + "]: " + this.message;
  };

  return AppError;
}(Error);

var err = new AppError("bad", 7);
console.log(err instanceof AppError, err instanceof Error, err.message, err.code);
console.log(typeof err.describe === "function" ? err.describe() : "no method");

var Stack = function (_Array) {
  _inherits(Stack, _Array);

  function Stack() {
    _classCallCheck(this, Stack);

    return _possibleConstructorReturn(this, _Array.apply(this, arguments));
  }

  Stack.prototype.peek = function peek() {
    return this[this.length - 1];
  };

  return Stack;
}(Array);

var s = new Stack();
s.push(1, 2, 3);
console.log(s.length, s instanceof Stack, typeof s.peek === "function" ? s.peek() : "no method");

var Registry = function (_Map) {
  _inherits(Registry, _Map);

  function Registry() {
    _classCallCheck(this, Registry);

    return _possibleConstructorReturn(this, _Map.apply(this, arguments));
  }

  Registry.prototype.register = function register(k, v) {
    this.set(k, v);
    return this;
  };

  return Registry;
}(Map);

var r = new Registry();
console.log(typeof r.register === "function" ? r.register("a", 1).get("a") : "no method", r instanceof Map);
