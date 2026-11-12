"use strict";
function _construct(Parent, args, Class) {
    if (_is_native_reflect_construct()) {
        _construct = Reflect.construct;
    } else {
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
    if (right != null && typeof Symbol !== "undefined" && right[Symbol.hasInstance]) {
        return !!right[Symbol.hasInstance](left);
    } else {
        return left instanceof right;
    }
}
function _is_native_function(fn) {
    return Function.toString.call(fn).indexOf("[native code]") !== -1;
}
function _set_prototype_of(o, p) {
    _set_prototype_of = Object.setPrototypeOf || function setPrototypeOf(o, p) {
        o.__proto__ = p;
        return o;
    };
    return _set_prototype_of(o, p);
}
function _wrap_native_super(Class) {
    var _cache = typeof Map === "function" ? new Map() : undefined;
    _wrap_native_super = function wrapNativeSuper(Class) {
        if (Class === null || !_is_native_function(Class)) return Class;
        if (typeof Class !== "function") {
            throw new TypeError("Super expression must either be null or a function");
        }
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
function _is_native_reflect_construct() {
    if (typeof Reflect === "undefined" || !Reflect.construct) return false;
    if (Reflect.construct.sham) return false;
    if (typeof Proxy === "function") return true;
    try {
        Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function() {}));
        return true;
    } catch (e) {
        return false;
    }
}
var AppError = /*#__PURE__*/ function(Error1) {
    "use strict";
    _inherits(AppError, Error1);
    function AppError(message, code) {
        var _this;
        _this = Error1.call(this, message) || this;
        _this.name = "AppError";
        _this.code = code;
        return _this;
    }
    var _proto = AppError.prototype;
    _proto.describe = function describe() {
        return this.name + "[" + this.code + "]: " + this.message;
    };
    return AppError;
}(_wrap_native_super(Error));
var err = new AppError("bad", 7);
console.log(_instanceof(err, AppError), _instanceof(err, Error), err.message, err.code);
console.log(typeof err.describe === "function" ? err.describe() : "no method");
var Stack = /*#__PURE__*/ function(Array1) {
    "use strict";
    _inherits(Stack, Array1);
    function Stack() {
        return Array1.apply(this, arguments);
    }
    var _proto = Stack.prototype;
    _proto.peek = function peek() {
        return this[this.length - 1];
    };
    return Stack;
}(_wrap_native_super(Array));
var s = new Stack();
s.push(1, 2, 3);
console.log(s.length, _instanceof(s, Stack), typeof s.peek === "function" ? s.peek() : "no method");
var Registry = /*#__PURE__*/ function(Map1) {
    "use strict";
    _inherits(Registry, Map1);
    function Registry() {
        return Map1.apply(this, arguments);
    }
    var _proto = Registry.prototype;
    _proto.register = function register(k, v) {
        this.set(k, v);
        return this;
    };
    return Registry;
}(_wrap_native_super(Map));
var r = new Registry();
console.log(typeof r.register === "function" ? r.register("a", 1).get("a") : "no method", _instanceof(r, Map));
