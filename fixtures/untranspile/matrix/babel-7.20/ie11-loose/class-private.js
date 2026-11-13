"use strict";

function _defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, _toPropertyKey(descriptor.key), descriptor); } }
function _createClass(Constructor, protoProps, staticProps) { if (protoProps) _defineProperties(Constructor.prototype, protoProps); if (staticProps) _defineProperties(Constructor, staticProps); Object.defineProperty(Constructor, "prototype", { writable: false }); return Constructor; }
function _toPropertyKey(arg) { var key = _toPrimitive(arg, "string"); return typeof key === "symbol" ? key : String(key); }
function _toPrimitive(input, hint) { if (typeof input !== "object" || input === null) return input; var prim = input[Symbol.toPrimitive]; if (prim !== undefined) { var res = prim.call(input, hint || "default"); if (typeof res !== "object") return res; throw new TypeError("@@toPrimitive must return a primitive value."); } return (hint === "string" ? String : Number)(input); }
function _checkInRHS(value) { if (Object(value) !== value) throw TypeError("right-hand side of 'in' should be an object, got " + (null !== value ? typeof value : "null")); return value; }
function _classPrivateFieldLooseBase(receiver, privateKey) { if (!Object.prototype.hasOwnProperty.call(receiver, privateKey)) { throw new TypeError("attempted to use private field on non-instance"); } return receiver; }
var id = 0;
function _classPrivateFieldLooseKey(name) { return "__private_" + id++ + "_" + name; }
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
  _createClass(Account, [{
    key: "balance",
    get: function get() {
      return _classPrivateFieldLooseBase(this, _balance)[_balance];
    }
  }]);
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
