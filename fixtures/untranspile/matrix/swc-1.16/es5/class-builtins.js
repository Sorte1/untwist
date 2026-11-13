"use strict";
function _assert_this_initialized(self) {
    if (self === void 0) throw new ReferenceError("this hasn't been initialised - super() hasn't been called");
    return self;
}
function _call_super(_this, derived, args) {
    derived = _get_prototype_of(derived);
    return _possible_constructor_return(_this, _is_native_reflect_construct() ? Reflect.construct(derived, args || [], _get_prototype_of(_this).constructor) : derived.apply(_this, args));
}
function _class_call_check(instance, Constructor) {
    if (!(instance instanceof Constructor)) throw new TypeError("Cannot call a class as a function");
}
function _construct(Parent, args, Class) {
    if (_is_native_reflect_construct()) _construct = Reflect.construct;
    else {
        _construct = function construct(Parent, args, Class) {
            var a = [
                null
            ];
            a.push.apply(a, args);
            var Constructor = Function.bind.apply(Parent, a);
            var instance = new Constructor();
            if (Class) _set_prototype_of(instance, Class.prototype);
            return instance;
        };
    }
    return _construct.apply(null, arguments);
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
function _get_prototype_of(o) {
    _get_prototype_of = Object.setPrototypeOf ? Object.getPrototypeOf : function getPrototypeOf(o) {
        return o.__proto__ || Object.getPrototypeOf(o);
    };
    return _get_prototype_of(o);
}
function _inherits(subClass, superClass) {
    if (typeof superClass !== "function" && superClass !== null) {
        throw new TypeError("Super expression must either be null or a function");
    }
    subClass.prototype = Object.create(superClass && superClass.prototype, {
        constructor: {
            value: subClass,
            writable: true,
            configurable: true
        }
    });
    if (superClass) _set_prototype_of(subClass, superClass);
}
function _instanceof(left, right) {
    "@swc/helpers - instanceof";
    if (right != null && typeof Symbol !== "undefined" && right[Symbol.hasInstance]) {
        return !!right[Symbol.hasInstance](left);
    } else return left instanceof right;
}
function _is_native_function(fn) {
    return Function.toString.call(fn).indexOf("[native code]") !== -1;
}
function _is_native_reflect_construct() {
    try {
        var result = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function() {}));
    } catch (_) {}
    return (_is_native_reflect_construct = function() {
        return !!result;
    })();
}
function _possible_constructor_return(self, call) {
    if (call && (_type_of(call) === "object" || typeof call === "function")) return call;
    return _assert_this_initialized(self);
}
function _set_prototype_of(o, p) {
    _set_prototype_of = Object.setPrototypeOf || function setPrototypeOf(o, p) {
        o.__proto__ = p;
        return o;
    };
    return _set_prototype_of(o, p);
}
function _type_of(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
}
function _wrap_native_super(Class) {
    var _cache = typeof Map === "function" ? new Map() : undefined;
    _wrap_native_super = function(Class) {
        if (Class === null || !_is_native_function(Class)) return Class;
        if (typeof Class !== "function") throw new TypeError("Super expression must either be null or a function");
        if (typeof _cache !== "undefined") {
            if (_cache.has(Class)) return _cache.get(Class);
            _cache.set(Class, Wrapper);
        }
        function Wrapper() {
            return _construct(Class, arguments, _get_prototype_of(this).constructor);
        }
        Wrapper.prototype = Object.create(Class.prototype, {
            constructor: {
                value: Wrapper,
                enumerable: false,
                writable: true,
                configurable: true
            }
        });
        return _set_prototype_of(Wrapper, Class);
    };
    return _wrap_native_super(Class);
}
var AppError = /*#__PURE__*/ function(Error1) {
    "use strict";
    _inherits(AppError, Error1);
    function AppError(message, code) {
        _class_call_check(this, AppError);
        var _this;
        _this = _call_super(this, AppError, [
            message
        ]);
        _this.name = "AppError";
        _this.code = code;
        return _this;
    }
    _create_class(AppError, [
        {
            key: "describe",
            value: function describe() {
                return this.name + "[" + this.code + "]: " + this.message;
            }
        }
    ]);
    return AppError;
}(_wrap_native_super(Error));
var err = new AppError("bad", 7);
console.log(_instanceof(err, AppError), _instanceof(err, Error), err.message, err.code);
console.log(typeof err.describe === "function" ? err.describe() : "no method");
var Stack = /*#__PURE__*/ function(Array1) {
    "use strict";
    _inherits(Stack, Array1);
    function Stack() {
        _class_call_check(this, Stack);
        return _call_super(this, Stack, arguments);
    }
    _create_class(Stack, [
        {
            key: "peek",
            value: function peek() {
                return this[this.length - 1];
            }
        }
    ]);
    return Stack;
}(_wrap_native_super(Array));
var s = new Stack();
s.push(1, 2, 3);
console.log(s.length, _instanceof(s, Stack), typeof s.peek === "function" ? s.peek() : "no method");
var Registry = /*#__PURE__*/ function(Map1) {
    "use strict";
    _inherits(Registry, Map1);
    function Registry() {
        _class_call_check(this, Registry);
        return _call_super(this, Registry, arguments);
    }
    _create_class(Registry, [
        {
            key: "register",
            value: function register(k, v) {
                this.set(k, v);
                return this;
            }
        }
    ]);
    return Registry;
}(_wrap_native_super(Map));
var r = new Registry();
console.log(typeof r.register === "function" ? r.register("a", 1).get("a") : "no method", _instanceof(r, Map));
