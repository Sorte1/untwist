"use strict";

function _defineAccessor(e, r, n, t) { var c = { configurable: !0, enumerable: !0 }; return c[e] = t, Object.defineProperty(r, n, c); }
function _checkInRHS(e) { if (Object(e) !== e) throw TypeError("right-hand side of 'in' should be an object, got " + (null !== e ? typeof e : "null")); return e; }
function _classPrivateFieldLooseBase(e, t) { if (!{}.hasOwnProperty.call(e, t)) throw new TypeError("attempted to use private field on non-instance"); return e; }
var id = 0;
function _classPrivateFieldLooseKey(e) { return "__private_" + id++ + "_" + e; }
var _balance = /*#__PURE__*/_classPrivateFieldLooseKey("balance");
var _count = /*#__PURE__*/_classPrivateFieldLooseKey("count");
var _log = /*#__PURE__*/_classPrivateFieldLooseKey("log");
var _record = /*#__PURE__*/_classPrivateFieldLooseKey("record");
var _summary = /*#__PURE__*/_classPrivateFieldLooseKey("summary");
var _secret = /*#__PURE__*/_classPrivateFieldLooseKey("secret");
var Account = /*#__PURE__*/function () {
  function Account(initial) {
    Object.defineProperty(this, _summary, {
      get: _get_summary,
      set: void 0
    });
    Object.defineProperty(this, _record, {
      value: _record2
    });
    Object.defineProperty(this, _balance, {
      writable: true,
      value: 0
    });
    Object.defineProperty(this, _log, {
      writable: true,
      value: []
    });
    _classPrivateFieldLooseBase(this, _balance)[_balance] = initial;
    _classPrivateFieldLooseBase(Account, _count)[_count]++;
  }
  var _proto = Account.prototype;
  _proto.deposit = function deposit(n) {
    _classPrivateFieldLooseBase(this, _balance)[_balance] += n;
    _classPrivateFieldLooseBase(this, _record)[_record]("+" + n);
    return this;
  };
  _defineAccessor("get", _proto, "balance", function () {
    return _classPrivateFieldLooseBase(this, _balance)[_balance];
  });
  _proto.history = function history() {
    return _classPrivateFieldLooseBase(this, _summary)[_summary];
  };
  Account.count = function count() {
    return _classPrivateFieldLooseBase(Account, _count)[_count];
  };
  Account.reveal = function reveal() {
    return _classPrivateFieldLooseBase(Account, _secret)[_secret]();
  };
  Account.isAccount = function isAccount(o) {
    return Object.prototype.hasOwnProperty.call(_checkInRHS(o), _balance);
  };
  return Account;
}();
function _record2(op) {
  _classPrivateFieldLooseBase(this, _log)[_log].push(op);
}
function _get_summary() {
  return _classPrivateFieldLooseBase(this, _log)[_log].join("|");
}
function _secret2() {
  return "s";
}
Object.defineProperty(Account, _secret, {
  value: _secret2
});
Object.defineProperty(Account, _count, {
  writable: true,
  value: 0
});
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
