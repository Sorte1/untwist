"use strict";
function _class_private_field_loose_base(receiver, privateKey) {
    if (!Object.prototype.hasOwnProperty.call(receiver, privateKey)) {
        throw new TypeError("attempted to use private field on non-instance");
    }
    return receiver;
}
var id = 0;
function _class_private_field_loose_key(name) {
    return "__private_" + id++ + "_" + name;
}
function _defineProperties(target, props) {
    for(var i = 0; i < props.length; i++){
        var descriptor = props[i];
        descriptor.enumerable = descriptor.enumerable || false;
        descriptor.configurable = true;
        if ("value" in descriptor) descriptor.writable = true;
        Object.defineProperty(target, descriptor.key, descriptor);
    }
}
function _create_class(Constructor, protoProps, staticProps) {
    if (protoProps) _defineProperties(Constructor.prototype, protoProps);
    if (staticProps) _defineProperties(Constructor, staticProps);
    return Constructor;
}
var _balance = /*#__PURE__*/ _class_private_field_loose_key("_balance"), _count = /*#__PURE__*/ _class_private_field_loose_key("_count"), _log = /*#__PURE__*/ _class_private_field_loose_key("_log"), _record = /*#__PURE__*/ _class_private_field_loose_key("_record"), _summary = /*#__PURE__*/ _class_private_field_loose_key("_summary"), _secret = /*#__PURE__*/ _class_private_field_loose_key("_secret");
var Account = /*#__PURE__*/ function() {
    "use strict";
    function Account(initial) {
        Object.defineProperty(this, _record, {
            value: record
        });
        Object.defineProperty(this, _summary, {
            get: get_summary,
            set: void 0
        });
        Object.defineProperty(this, _balance, {
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, _log, {
            writable: true,
            value: void 0
        });
        _class_private_field_loose_base(this, _balance)[_balance] = 0;
        _class_private_field_loose_base(this, _log)[_log] = [];
        _class_private_field_loose_base(this, _balance)[_balance] = initial;
        _class_private_field_loose_base(Account, _count)[_count]++;
    }
    var _proto = Account.prototype;
    _proto.deposit = function deposit(n) {
        _class_private_field_loose_base(this, _balance)[_balance] += n;
        _class_private_field_loose_base(this, _record)[_record]("+" + n);
        return this;
    };
    _proto.history = function history() {
        return _class_private_field_loose_base(this, _summary)[_summary];
    };
    Account.count = function count() {
        return _class_private_field_loose_base(Account, _count)[_count];
    };
    Account.reveal = function reveal() {
        return _class_private_field_loose_base(Account, _secret)[_secret]();
    };
    Account.isAccount = function isAccount(o) {
        return _balance.has(o);
    };
    _create_class(Account, [
        {
            key: "balance",
            get: function get() {
                return _class_private_field_loose_base(this, _balance)[_balance];
            }
        }
    ]);
    return Account;
}();
Object.defineProperty(Account, _secret, {
    value: secret
});
Object.defineProperty(Account, _count, {
    writable: true,
    value: 0
});
function record(op) {
    _class_private_field_loose_base(this, _log)[_log].push(op);
}
function get_summary() {
    return _class_private_field_loose_base(this, _log)[_log].join("|");
}
function secret() {
    return "s";
}
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
