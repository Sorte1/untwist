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
function _define_property(obj, key, value) {
    if (key in obj) {
        Object.defineProperty(obj, key, {
            value: value,
            enumerable: true,
            configurable: true,
            writable: true
        });
    } else obj[key] = value;
    return obj;
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
var seq = 0;
var _ref = "dyn" + seq;
var Counter = function Counter(step) {
    "use strict";
    var _this = this;
    _class_call_check(this, Counter);
    _define_property(this, "count", 0);
    _define_property(this, "step", 1);
    _define_property(this, "id", ++seq);
    _define_property(this, "label", "c" + this.id);
    _define_property(this, _ref, "dynamic");
    _define_property(this, "inc", function() {
        _this.count += _this.step;
        return _this;
    });
    if (step) this.step = step;
    Counter.instances++;
};
_define_property(Counter, "instances", 0);
_define_property(Counter, "created", Counter.instances + 1);
var c = new Counter(2);
c.inc().inc();
console.log(c.count, c.label, c.dyn1, Counter.instances, Counter.created);
var inc = c.inc;
inc();
console.log(c.count, Object.keys(c).join(","));
var Base = /*#__PURE__*/ function() {
    "use strict";
    function Base() {
        _class_call_check(this, Base);
        this.setup();
    }
    _create_class(Base, [
        {
            key: "setup",
            value: function setup() {}
        }
    ]);
    return Base;
}();
var Derived = /*#__PURE__*/ function(Base) {
    "use strict";
    _inherits(Derived, Base);
    function Derived() {
        _class_call_check(this, Derived);
        var _this;
        _this = _call_super(this, Derived, arguments), _define_property(_this, "value", 5);
        return _this;
    }
    _create_class(Derived, [
        {
            key: "setup",
            value: function setup() {
                this.seen = this.value;
            }
        }
    ]);
    return Derived;
}(Base);
var dd = new Derived();
console.log(dd.seen, dd.value);
var Shadow = /*#__PURE__*/ function() {
    "use strict";
    function Shadow() {
        _class_call_check(this, Shadow);
    }
    _create_class(Shadow, [
        {
            key: "x",
            set: function set(v) {
                this.setterRan = true;
            }
        }
    ]);
    return Shadow;
}();
var Defines = /*#__PURE__*/ function(Shadow) {
    "use strict";
    _inherits(Defines, Shadow);
    function Defines() {
        _class_call_check(this, Defines);
        var _this;
        _this = _call_super(this, Defines, arguments), _define_property(_this, "x", 1);
        return _this;
    }
    return Defines;
}(Shadow);
var df = new Defines();
console.log(df.x, df.setterRan === true);
