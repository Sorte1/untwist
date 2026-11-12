"use strict";
var __classPrivateFieldSet = (this && this.__classPrivateFieldSet) || function (receiver, state, value, kind, f) {
    if (kind === "m") throw new TypeError("Private method is not writable");
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a setter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot write private member to an object whose class did not declare it");
    return (kind === "a" ? f.call(receiver, value) : f ? f.value = value : state.set(receiver, value)), value;
};
var __classPrivateFieldGet = (this && this.__classPrivateFieldGet) || function (receiver, state, kind, f) {
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a getter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
    return kind === "m" ? f : kind === "a" ? f.call(receiver) : f ? f.value : state.get(receiver);
};
var __classPrivateFieldIn = (this && this.__classPrivateFieldIn) || function(state, receiver) {
    if (receiver === null || (typeof receiver !== "object" && typeof receiver !== "function")) throw new TypeError("Cannot use 'in' operator on non-object");
    return typeof state === "function" ? receiver === state : state.has(receiver);
};
var Account = /** @class */ (function () {
    function Account(initial) {
        var _b, _c;
        _Account_instances.add(this);
        _Account_balance.set(this, 0);
        _Account_log.set(this, []);
        __classPrivateFieldSet(this, _Account_balance, initial, "f");
        __classPrivateFieldSet(_b = Account, _a, (_c = __classPrivateFieldGet(_b, _a, "f", _Account_count), _c++, _c), "f", _Account_count);
    }
    Account.prototype.deposit = function (n) {
        __classPrivateFieldSet(this, _Account_balance, __classPrivateFieldGet(this, _Account_balance, "f") + n, "f");
        __classPrivateFieldGet(this, _Account_instances, "m", _Account_record).call(this, "+" + n);
        return this;
    };
    Object.defineProperty(Account.prototype, "balance", {
        get: function () {
            return __classPrivateFieldGet(this, _Account_balance, "f");
        },
        enumerable: false,
        configurable: true
    });
    Account.prototype.history = function () {
        return __classPrivateFieldGet(this, _Account_instances, "a", _Account_summary_get);
    };
    Account.count = function () {
        return __classPrivateFieldGet(Account, _a, "f", _Account_count);
    };
    Account.reveal = function () {
        return __classPrivateFieldGet(Account, _a, "m", _Account_secret).call(Account);
    };
    Account.isAccount = function (o) {
        return __classPrivateFieldIn(_Account_balance, o);
    };
    var _Account_instances, _a, _Account_balance, _Account_count, _Account_log, _Account_record, _Account_summary_get, _Account_secret;
    _a = Account, _Account_balance = new WeakMap(), _Account_log = new WeakMap(), _Account_instances = new WeakSet(), _Account_record = function _Account_record(op) {
        __classPrivateFieldGet(this, _Account_log, "f").push(op);
    }, _Account_summary_get = function _Account_summary_get() {
        return __classPrivateFieldGet(this, _Account_log, "f").join("|");
    }, _Account_secret = function _Account_secret() {
        return "s";
    };
    _Account_count = { value: 0 };
    return Account;
}());
var a = new Account(10);
a.deposit(5).deposit(1);
new Account(0);
console.log(a.balance, a.history(), Account.count(), Account.reveal());
console.log(Account.isAccount(a), Account.isAccount({}), Object.keys(a).length);
try {
    Account.prototype.deposit.call({}, 1);
}
catch (e) {
    console.log(e.constructor.name);
}
