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
function _get(target, property, receiver) {
    if (typeof Reflect !== "undefined" && Reflect.get) _get = Reflect.get;
    else {
        _get = function get(target, property, receiver) {
            var base = _super_prop_base(target, property);
            if (!base) return;
            var desc = Object.getOwnPropertyDescriptor(base, property);
            if (desc.get) return desc.get.call(receiver || target);
            return desc.value;
        };
    }
    return _get(target, property, receiver || target);
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
function _super_prop_base(object, property) {
    while(!Object.prototype.hasOwnProperty.call(object, property)){
        object = _get_prototype_of(object);
        if (object === null) break;
    }
    return object;
}
function _type_of(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
}
var Base = /*#__PURE__*/ function() {
    "use strict";
    function Base(v) {
        _class_call_check(this, Base);
        this.v = v;
    }
    _create_class(Base, [
        {
            key: "val",
            get: function get() {
                return this.v;
            },
            set: function set(x) {
                this.v = x * 2;
            }
        },
        {
            key: "greet",
            value: function greet(who) {
                return "hi " + who + " " + this.v;
            }
        },
        {
            key: "comp" + "uted",
            value: function value() {
                return "c";
            }
        }
    ], [
        {
            key: "make",
            value: function make(v) {
                return new this(v);
            }
        },
        {
            key: "kind",
            get: function get() {
                return "base";
            }
        }
    ]);
    return Base;
}();
var Child = /*#__PURE__*/ function(Base) {
    "use strict";
    _inherits(Child, Base);
    function Child(v, w) {
        _class_call_check(this, Child);
        var _this;
        _this = _call_super(this, Child, [
            v
        ]);
        _this.w = w;
        return _this;
    }
    _create_class(Child, [
        {
            key: "greet",
            value: function greet(who) {
                return _get(_get_prototype_of(Child.prototype), "greet", this).call(this, who).toUpperCase() + this.w;
            }
        }
    ], [
        {
            key: "make",
            value: function make(v) {
                return _get(_get_prototype_of(Child), "make", this).call(this, v);
            }
        }
    ]);
    return Child;
}(Base);
var c = new Child(1, 2);
console.log(c.greet("x"), c.val, (c.val = 5, c.v), c.computed());
console.log(_instanceof(Child.make(3), Child), Child.kind, Object.getPrototypeOf(Child) === Base);
console.log(Base.name, Child.name, typeof Base === "undefined" ? "undefined" : _type_of(Base));
console.log(Object.getOwnPropertyNames(Base.prototype).sort().join(","), Object.keys(Base.prototype).length);
var d = Object.getOwnPropertyDescriptor(Base.prototype, "greet");
console.log(d.enumerable, d.writable, d.configurable);
console.log(Object.getOwnPropertyDescriptor(Base, "make").enumerable);
try {
    Base(1);
} catch (e) {
    console.log(e.constructor.name);
}
var Anon = function Anon() {
    "use strict";
    _class_call_check(this, Anon);
};
console.log(Anon.name);
var Counter = /*#__PURE__*/ function() {
    "use strict";
    function Counter() {
        _class_call_check(this, Counter);
        this.n = 0;
    }
    _create_class(Counter, [
        {
            key: "inc",
            value: function inc() {
                this.n++;
                return this;
            }
        }
    ]);
    return Counter;
}();
console.log(new Counter().inc().inc().n);
function Legacy(x) {
    this.x = x;
}
Legacy.prototype.get = function() {
    return this.x;
};
var FromLegacy = /*#__PURE__*/ function(Legacy) {
    "use strict";
    _inherits(FromLegacy, Legacy);
    function FromLegacy() {
        _class_call_check(this, FromLegacy);
        return _call_super(this, FromLegacy, arguments);
    }
    _create_class(FromLegacy, [
        {
            key: "get",
            value: function get() {
                return _get(_get_prototype_of(FromLegacy.prototype), "get", this).call(this) + 1;
            }
        }
    ]);
    return FromLegacy;
}(Legacy);
console.log(new FromLegacy(4).get());
