"use strict";
function _check_private_redeclaration(obj, privateCollection) {
    if (privateCollection.has(obj)) {
        throw new TypeError("Cannot initialize the same private elements twice on an object");
    }
}
function _class_apply_descriptor_get(receiver, descriptor) {
    if (descriptor.get) {
        return descriptor.get.call(receiver);
    }
    return descriptor.value;
}
function _class_apply_descriptor_set(receiver, descriptor, value) {
    if (descriptor.set) {
        descriptor.set.call(receiver, value);
    } else {
        if (!descriptor.writable) {
            throw new TypeError("attempted to set read only private field");
        }
        descriptor.value = value;
    }
}
function _class_apply_descriptor_update(receiver, descriptor) {
    if (descriptor.set) {
        if (!descriptor.get) {
            throw new TypeError("attempted to read set only private field");
        }
        if (!("__destrWrapper" in descriptor)) {
            descriptor.__destrWrapper = {
                set value (v){
                    descriptor.set.call(receiver, v);
                },
                get value () {
                    return descriptor.get.call(receiver);
                }
            };
        }
        return descriptor.__destrWrapper;
    } else {
        if (!descriptor.writable) {
            throw new TypeError("attempted to set read only private field");
        }
        return descriptor;
    }
}
function _class_call_check(instance, Constructor) {
    if (!(instance instanceof Constructor)) {
        throw new TypeError("Cannot call a class as a function");
    }
}
function _class_check_private_static_field_descriptor(descriptor, action) {
    if (descriptor === undefined) {
        throw new TypeError("attempted to " + action + " private static field before its declaration");
    }
}
function _class_extract_field_descriptor(receiver, privateMap, action) {
    if (!privateMap.has(receiver)) {
        throw new TypeError("attempted to " + action + " private field on non-instance");
    }
    return privateMap.get(receiver);
}
function _class_private_field_get(receiver, privateMap) {
    var descriptor = _class_extract_field_descriptor(receiver, privateMap, "get");
    return _class_apply_descriptor_get(receiver, descriptor);
}
function _class_private_field_init(obj, privateMap, value) {
    _check_private_redeclaration(obj, privateMap);
    privateMap.set(obj, value);
}
function _class_private_field_set(receiver, privateMap, value) {
    var descriptor = _class_extract_field_descriptor(receiver, privateMap, "set");
    _class_apply_descriptor_set(receiver, descriptor, value);
    return value;
}
function _class_private_method_get(receiver, privateSet, fn) {
    if (!privateSet.has(receiver)) {
        throw new TypeError("attempted to get private field on non-instance");
    }
    return fn;
}
function _class_private_method_init(obj, privateSet) {
    _check_private_redeclaration(obj, privateSet);
    privateSet.add(obj);
}
function _class_static_private_field_spec_get(receiver, classConstructor, descriptor) {
    _class_check_private_static_access(receiver, classConstructor);
    _class_check_private_static_field_descriptor(descriptor, "get");
    return _class_apply_descriptor_get(receiver, descriptor);
}
function _class_static_private_field_update(receiver, classConstructor, descriptor) {
    _class_check_private_static_access(receiver, classConstructor);
    _class_check_private_static_field_descriptor(descriptor, "update");
    return _class_apply_descriptor_update(receiver, descriptor);
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
function _class_static_private_method_get(receiver, classConstructor, method) {
    _class_check_private_static_access(receiver, classConstructor);
    return method;
}
function _class_check_private_static_access(receiver, classConstructor) {
    if (receiver !== classConstructor) {
        throw new TypeError("Private static access of wrong provenance");
    }
}
var _balance = /*#__PURE__*/ new WeakMap(), _log = /*#__PURE__*/ new WeakMap(), _record = /*#__PURE__*/ new WeakSet(), _summary = /*#__PURE__*/ new WeakMap();
var Account = /*#__PURE__*/ function() {
    "use strict";
    function Account(initial) {
        _class_call_check(this, Account);
        _class_private_method_init(this, _record);
        _class_private_field_init(this, _summary, {
            get: get_summary,
            set: void 0
        });
        _class_private_field_init(this, _balance, {
            writable: true,
            value: 0
        });
        _class_private_field_init(this, _log, {
            writable: true,
            value: []
        });
        _class_private_field_set(this, _balance, initial);
        _class_static_private_field_update(Account, Account, _count).value++;
    }
    _create_class(Account, [
        {
            key: "deposit",
            value: function deposit(n) {
                _class_private_field_set(this, _balance, _class_private_field_get(this, _balance) + n);
                _class_private_method_get(this, _record, record).call(this, "+" + n);
                return this;
            }
        },
        {
            key: "balance",
            get: function get() {
                return _class_private_field_get(this, _balance);
            }
        },
        {
            key: "history",
            value: function history() {
                return _class_private_field_get(this, _summary);
            }
        }
    ], [
        {
            key: "count",
            value: function count() {
                return _class_static_private_field_spec_get(Account, Account, _count);
            }
        },
        {
            key: "reveal",
            value: function reveal() {
                return _class_static_private_method_get(Account, Account, secret).call(Account);
            }
        },
        {
            key: "isAccount",
            value: function isAccount(o) {
                return _balance.has(o);
            }
        }
    ]);
    return Account;
}();
var _count = {
    writable: true,
    value: 0
};
function record(op) {
    _class_private_field_get(this, _log).push(op);
}
function get_summary() {
    return _class_private_field_get(this, _log).join("|");
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
