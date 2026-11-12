"use strict";

function _typeof(obj) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (obj) { return typeof obj; } : function (obj) { return obj && "function" == typeof Symbol && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }, _typeof(obj); }
function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }
function _defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, _toPropertyKey(descriptor.key), descriptor); } }
function _createClass(Constructor, protoProps, staticProps) { if (protoProps) _defineProperties(Constructor.prototype, protoProps); if (staticProps) _defineProperties(Constructor, staticProps); Object.defineProperty(Constructor, "prototype", { writable: false }); return Constructor; }
function _toPropertyKey(arg) { var key = _toPrimitive(arg, "string"); return _typeof(key) === "symbol" ? key : String(key); }
function _toPrimitive(input, hint) { if (_typeof(input) !== "object" || input === null) return input; var prim = input[Symbol.toPrimitive]; if (prim !== undefined) { var res = prim.call(input, hint || "default"); if (_typeof(res) !== "object") return res; throw new TypeError("@@toPrimitive must return a primitive value."); } return (hint === "string" ? String : Number)(input); }
function _classPrivateMethodInitSpec(obj, privateSet) { _checkPrivateRedeclaration(obj, privateSet); privateSet.add(obj); }
function _classPrivateFieldInitSpec(obj, privateMap, value) { _checkPrivateRedeclaration(obj, privateMap); privateMap.set(obj, value); }
function _checkPrivateRedeclaration(obj, privateCollection) { if (privateCollection.has(obj)) { throw new TypeError("Cannot initialize the same private elements twice on an object"); } }
function _checkInRHS(value) { if (Object(value) !== value) throw TypeError("right-hand side of 'in' should be an object, got " + (null !== value ? _typeof(value) : "null")); return value; }
function _classStaticPrivateMethodGet(receiver, classConstructor, method) { _classCheckPrivateStaticAccess(receiver, classConstructor); return method; }
function _classPrivateMethodGet(receiver, privateSet, fn) { if (!privateSet.has(receiver)) { throw new TypeError("attempted to get private field on non-instance"); } return fn; }
function _classPrivateFieldGet(receiver, privateMap) { var descriptor = _classExtractFieldDescriptor(receiver, privateMap, "get"); return _classApplyDescriptorGet(receiver, descriptor); }
function _classStaticPrivateFieldSpecSet(receiver, classConstructor, descriptor, value) { _classCheckPrivateStaticAccess(receiver, classConstructor); _classCheckPrivateStaticFieldDescriptor(descriptor, "set"); _classApplyDescriptorSet(receiver, descriptor, value); return value; }
function _classStaticPrivateFieldSpecGet(receiver, classConstructor, descriptor) { _classCheckPrivateStaticAccess(receiver, classConstructor); _classCheckPrivateStaticFieldDescriptor(descriptor, "get"); return _classApplyDescriptorGet(receiver, descriptor); }
function _classCheckPrivateStaticFieldDescriptor(descriptor, action) { if (descriptor === undefined) { throw new TypeError("attempted to " + action + " private static field before its declaration"); } }
function _classCheckPrivateStaticAccess(receiver, classConstructor) { if (receiver !== classConstructor) { throw new TypeError("Private static access of wrong provenance"); } }
function _classApplyDescriptorGet(receiver, descriptor) { if (descriptor.get) { return descriptor.get.call(receiver); } return descriptor.value; }
function _classPrivateFieldSet(receiver, privateMap, value) { var descriptor = _classExtractFieldDescriptor(receiver, privateMap, "set"); _classApplyDescriptorSet(receiver, descriptor, value); return value; }
function _classExtractFieldDescriptor(receiver, privateMap, action) { if (!privateMap.has(receiver)) { throw new TypeError("attempted to " + action + " private field on non-instance"); } return privateMap.get(receiver); }
function _classApplyDescriptorSet(receiver, descriptor, value) { if (descriptor.set) { descriptor.set.call(receiver, value); } else { if (!descriptor.writable) { throw new TypeError("attempted to set read only private field"); } descriptor.value = value; } }
var _balance = /*#__PURE__*/new WeakMap();
var _log = /*#__PURE__*/new WeakMap();
var _record = /*#__PURE__*/new WeakSet();
var _summary = /*#__PURE__*/new WeakMap();
var Account = /*#__PURE__*/function () {
  function Account(initial) {
    var _Account$count, _Account$count2;
    _classCallCheck(this, Account);
    _classPrivateFieldInitSpec(this, _summary, {
      get: _get_summary,
      set: void 0
    });
    _classPrivateMethodInitSpec(this, _record);
    _classPrivateFieldInitSpec(this, _balance, {
      writable: true,
      value: 0
    });
    _classPrivateFieldInitSpec(this, _log, {
      writable: true,
      value: []
    });
    _classPrivateFieldSet(this, _balance, initial);
    _classStaticPrivateFieldSpecSet(Account, Account, _count, (_Account$count = _classStaticPrivateFieldSpecGet(Account, Account, _count), _Account$count2 = _Account$count++, _Account$count)), _Account$count2;
  }
  _createClass(Account, [{
    key: "deposit",
    value: function deposit(n) {
      _classPrivateFieldSet(this, _balance, _classPrivateFieldGet(this, _balance) + n);
      _classPrivateMethodGet(this, _record, _record2).call(this, "+" + n);
      return this;
    }
  }, {
    key: "balance",
    get: function get() {
      return _classPrivateFieldGet(this, _balance);
    }
  }, {
    key: "history",
    value: function history() {
      return _classPrivateFieldGet(this, _summary);
    }
  }], [{
    key: "count",
    value: function count() {
      return _classStaticPrivateFieldSpecGet(Account, Account, _count);
    }
  }, {
    key: "reveal",
    value: function reveal() {
      return _classStaticPrivateMethodGet(Account, Account, _secret).call(Account);
    }
  }, {
    key: "isAccount",
    value: function isAccount(o) {
      return _balance.has(_checkInRHS(o));
    }
  }]);
  return Account;
}();
function _record2(op) {
  _classPrivateFieldGet(this, _log).push(op);
}
function _get_summary() {
  return _classPrivateFieldGet(this, _log).join("|");
}
function _secret() {
  return "s";
}
var _count = {
  writable: true,
  value: 0
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
