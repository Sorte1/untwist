var __typeError = (msg) => {
  throw TypeError(msg);
};
var __accessCheck = (obj, member, msg) => member.has(obj) || __typeError("Cannot " + msg);
var __privateIn = (member, obj) => Object(obj) !== obj ? __typeError('Cannot use the "in" operator on this value') : member.has(obj);
var __privateGet = (obj, member, getter) => (__accessCheck(obj, member, "read from private field"), getter ? getter.call(obj) : member.get(obj));
var __privateAdd = (obj, member, value) => member.has(obj) ? __typeError("Cannot add the same private member more than once") : member instanceof WeakSet ? member.add(obj) : member.set(obj, value);
var __privateSet = (obj, member, value, setter) => (__accessCheck(obj, member, "write to private field"), setter ? setter.call(obj, value) : member.set(obj, value), value);
var __privateMethod = (obj, member, method) => (__accessCheck(obj, member, "access private method"), method);
var __privateWrapper = (obj, member, setter, getter) => ({
  set _(value) {
    __privateSet(obj, member, value, setter);
  },
  get _() {
    return __privateGet(obj, member, getter);
  }
});
var _balance, _count, _log, _Account_instances, record_fn, summary_get, _Account_static, secret_fn;
const _Account = class _Account {
  constructor(initial) {
    __privateAdd(this, _Account_instances);
    __privateAdd(this, _balance, 0);
    __privateAdd(this, _log, []);
    __privateSet(this, _balance, initial);
    __privateWrapper(_Account, _count)._++;
  }
  deposit(n) {
    __privateSet(this, _balance, __privateGet(this, _balance) + n);
    __privateMethod(this, _Account_instances, record_fn).call(this, "+" + n);
    return this;
  }
  get balance() {
    return __privateGet(this, _balance);
  }
  history() {
    return __privateGet(this, _Account_instances, summary_get);
  }
  static count() {
    return __privateGet(_Account, _count);
  }
  static reveal() {
    var _a;
    return __privateMethod(_a = _Account, _Account_static, secret_fn).call(_a);
  }
  static isAccount(o) {
    return __privateIn(_balance, o);
  }
};
_balance = new WeakMap();
_count = new WeakMap();
_log = new WeakMap();
_Account_instances = new WeakSet();
record_fn = function(op) {
  __privateGet(this, _log).push(op);
};
summary_get = function() {
  return __privateGet(this, _log).join("|");
};
_Account_static = new WeakSet();
secret_fn = function() {
  return "s";
};
__privateAdd(_Account, _Account_static);
__privateAdd(_Account, _count, 0);
let Account = _Account;
const a = new Account(10);
a.deposit(5).deposit(1);
new Account(0);
console.log(a.balance, a.history(), Account.count(), Account.reveal());
console.log(Account.isAccount(a), Account.isAccount({}), Object.keys(a).length);
try {
  Account.prototype.deposit.call({}, 1);
} catch (e) {
  console.log(e.constructor.name);
}
