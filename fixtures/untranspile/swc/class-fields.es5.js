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
var counter = 0;
var _ref = "k" + counter;
var Fields = /*#__PURE__*/ function() {
    "use strict";
    function Fields() {
        _class_call_check(this, Fields);
        _define_property(this, "a", 1);
        _define_property(this, "b", this.a + 1);
        _define_property(this, _ref, "dyn");
    }
    _create_class(Fields, [
        {
            key: "m",
            value: function m() {
                return this.a + this.b;
            }
        }
    ]);
    return Fields;
}();
_define_property(Fields, "s", ++counter);
_define_property(Fields, "t", Fields.s + 1);
var f = new Fields();
console.log(f.a, f.b, Fields.s, Fields.t, f.k1, f.m(), Object.keys(f).join(","));
var Parent = /*#__PURE__*/ function() {
    "use strict";
    function Parent() {
        _class_call_check(this, Parent);
        this.init();
    }
    _create_class(Parent, [
        {
            key: "init",
            value: function init() {}
        }
    ]);
    return Parent;
}();
var Kid = /*#__PURE__*/ function(Parent) {
    "use strict";
    _inherits(Kid, Parent);
    function Kid() {
        _class_call_check(this, Kid);
        var _this;
        _this = _call_super(this, Kid, arguments), _define_property(_this, "x", 1);
        return _this;
    }
    _create_class(Kid, [
        {
            key: "init",
            value: function init() {
                this.seen = this.x;
            }
        }
    ]);
    return Kid;
}(Parent);
console.log(new Kid().seen, new Kid().x);
var WithStaticBlock = function WithStaticBlock() {
    "use strict";
    _class_call_check(this, WithStaticBlock);
};
_define_property(WithStaticBlock, "z", void 0);
WithStaticBlock.z = 42;
console.log(WithStaticBlock.z);
var Arrowed = function Arrowed() {
    "use strict";
    var _this = this;
    _class_call_check(this, Arrowed);
    _define_property(this, "name", "arrowed");
    _define_property(this, "get", function() {
        return _this.name;
    });
};
var get = new Arrowed().get;
console.log(get());
