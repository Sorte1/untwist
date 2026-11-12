function _classPrivateMethodInitSpec(t, a) { _checkPrivateRedeclaration(t, a), a.add(t); }
function _classPrivateFieldInitSpec(t, i, a) { _checkPrivateRedeclaration(t, i), i.set(t, a); }
function _checkPrivateRedeclaration(e, t) { if (t.has(e)) throw new TypeError("Cannot initialize the same private elements twice on an object"); }
function _checkInRHS(e) { if (Object(e) !== e) throw TypeError("right-hand side of 'in' should be an object, got " + (null !== e ? typeof e : "null")); return e; }
function _classPrivateGetter(t, e, a) { return a(_assertClassBrand(t, e)); }
function _classPrivateFieldGet(e, r) { return e.get(_assertClassBrand(e, r)); }
function _classPrivateFieldSet(e, r, s) { return e.set(_assertClassBrand(e, r), s), s; }
function _assertClassBrand(e, t, n) { if ("function" == typeof e ? e === t : e.has(t)) return arguments.length < 3 ? t : n; throw new TypeError("Private element is not present on this object"); }
var _balance = /*#__PURE__*/new WeakMap();
var _log = /*#__PURE__*/new WeakMap();
var _Account_brand = /*#__PURE__*/new WeakSet();
class Account {
  constructor(initial) {
    var _Account$count, _Account$count2;
    _classPrivateMethodInitSpec(this, _Account_brand);
    _classPrivateFieldInitSpec(this, _balance, 0);
    _classPrivateFieldInitSpec(this, _log, []);
    _classPrivateFieldSet(_balance, this, initial);
    _count._ = (_Account$count = _count._, _Account$count2 = _Account$count++, _Account$count), _Account$count2;
  }
  deposit(n) {
    _classPrivateFieldSet(_balance, this, _classPrivateFieldGet(_balance, this) + n);
    _assertClassBrand(_Account_brand, this, _record).call(this, "+" + n);
    return this;
  }
  get balance() {
    return _classPrivateFieldGet(_balance, this);
  }
  history() {
    return _classPrivateGetter(_Account_brand, this, _get_summary);
  }
  static count() {
    return _count._;
  }
  static reveal() {
    return _secret.call(Account);
  }
  static isAccount(o) {
    return _balance.has(_checkInRHS(o));
  }
}
function _record(op) {
  _classPrivateFieldGet(_log, this).push(op);
}
function _get_summary(_this) {
  return _classPrivateFieldGet(_log, _this).join("|");
}
function _secret() {
  return "s";
}
var _count = {
  _: 0
};
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
