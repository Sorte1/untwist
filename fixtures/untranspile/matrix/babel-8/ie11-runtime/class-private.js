(function () {
  'use strict';

  function _classCallCheck(a, n) {
    if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function");
  }

  function _typeof(o) {
    "@babel/helpers - typeof";

    return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) {
      return typeof o;
    } : function (o) {
      return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o;
    }, _typeof(o);
  }

  function toPrimitive(t, e) {
    if ("object" != _typeof(t) || !t) return t;
    var r;
    if ("undefined" != typeof Symbol && void 0 !== (r = t[Symbol.toPrimitive])) {
      var i = r.call(t, e);
      if ("object" != _typeof(i)) return i;
      throw new TypeError("@@toPrimitive must return a primitive value.");
    }
    return (String )(t);
  }

  function toPropertyKey(r) {
    var o = toPrimitive(r, "string");
    return "symbol" == _typeof(o) ? o : o + "";
  }

  function _defineProperties(r, t) {
    for (var o = 0; o < t.length; o++) {
      var i = t[o];
      i.enumerable = i.enumerable || false, i.configurable = true, "value" in i && (i.writable = true), Object.defineProperty(r, toPropertyKey(i.key), i);
    }
  }
  function _createClass(e, r, t) {
    return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", {
      writable: false
    }), e;
  }

  function _checkPrivateRedeclaration(e, t) {
    if (t.has(e)) throw new TypeError("Cannot initialize the same private elements twice on an object");
  }

  function _classPrivateMethodInitSpec(t, a) {
    _checkPrivateRedeclaration(t, a), a.add(t);
  }

  function _classPrivateFieldInitSpec(t, i, a) {
    _checkPrivateRedeclaration(t, i), i.set(t, a);
  }

  function _checkInRHS(e) {
    if (Object(e) !== e) throw TypeError("right-hand side of 'in' should be an object, got " + (null !== e ? _typeof(e) : "null"));
    return e;
  }

  function _assertClassBrand(e, t, n) {
    if ("function" == typeof e ? e === t : e.has(t)) return arguments.length < 3 ? t : n;
    throw new TypeError("Private element is not present on this object");
  }

  function _classPrivateGetter(t, e, a) {
    return a(_assertClassBrand(t, e));
  }

  function _classPrivateFieldGet2(e, r) {
    return e.get(_assertClassBrand(e, r));
  }

  function _classPrivateFieldSet2(e, r, s) {
    return e.set(_assertClassBrand(e, r), s), s;
  }

  var _balance = /*#__PURE__*/new WeakMap();
  var _log = /*#__PURE__*/new WeakMap();
  var _Account_brand = /*#__PURE__*/new WeakSet();
  var Account = /*#__PURE__*/function () {
    function Account(initial) {
      var _Account$count;
      _classCallCheck(this, Account);
      _classPrivateMethodInitSpec(this, _Account_brand);
      _classPrivateFieldInitSpec(this, _balance, 0);
      _classPrivateFieldInitSpec(this, _log, []);
      _classPrivateFieldSet2(_balance, this, initial);
      _count._ = (_Account$count = _count._, _Account$count++, _Account$count);
    }
    return _createClass(Account, [{
      key: "deposit",
      value: function deposit(n) {
        _classPrivateFieldSet2(_balance, this, _classPrivateFieldGet2(_balance, this) + n);
        _assertClassBrand(_Account_brand, this, _record).call(this, "+" + n);
        return this;
      }
    }, {
      key: "balance",
      get: function get() {
        return _classPrivateFieldGet2(_balance, this);
      }
    }, {
      key: "history",
      value: function history() {
        return _classPrivateGetter(_Account_brand, this, _get_summary);
      }
    }], [{
      key: "count",
      value: function count() {
        return _count._;
      }
    }, {
      key: "reveal",
      value: function reveal() {
        return _secret.call(Account);
      }
    }, {
      key: "isAccount",
      value: function isAccount(o) {
        return _balance.has(_checkInRHS(o));
      }
    }]);
  }();
  function _record(op) {
    _classPrivateFieldGet2(_log, this).push(op);
  }
  function _get_summary(_this) {
    return _classPrivateFieldGet2(_log, _this).join("|");
  }
  function _secret() {
    return "s";
  }
  var _count = {
    _: 0
  };
  var a = new Account(10);
  a.deposit(5).deposit(1);
  new Account(0);
  console.log(a.balance, a.history(), Account.count(), Account.reveal());
  console.log(Account.isAccount(a), Account.isAccount({}), Object.keys(a).length);
  try {
    Account.prototype.deposit.call({}, 1);
  } catch (e) {
    console.log(e.constructor.name);
  }

})();
