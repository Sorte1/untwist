function _array_like_to_array(arr, len) {
    if (len == null || len > arr.length) len = arr.length;
    for(var i = 0, arr2 = new Array(len); i < len; i++)arr2[i] = arr[i];
    return arr2;
}
function _array_without_holes(arr) {
    if (Array.isArray(arr)) return _array_like_to_array(arr);
}
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
function _iterable_to_array(iter) {
    if (typeof Symbol !== "undefined" && iter[Symbol.iterator] != null || iter["@@iterator"] != null) {
        return Array.from(iter);
    }
}
function _non_iterable_spread() {
    throw new TypeError("Invalid attempt to spread non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
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
function _to_consumable_array(arr) {
    return _array_without_holes(arr) || _iterable_to_array(arr) || _unsupported_iterable_to_array(arr) || _non_iterable_spread();
}
function _type_of(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
}
function _unsupported_iterable_to_array(o, minLen) {
    if (!o) return;
    if (typeof o === "string") return _array_like_to_array(o, minLen);
    var n = Object.prototype.toString.call(o).slice(8, -1);
    if (n === "Object" && o.constructor) n = o.constructor.name;
    if (n === "Map" || n === "Set") return Array.from(o);
    if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return _array_like_to_array(o, minLen);
}
var Shape = function Shape(kind) {
    "use strict";
    _class_call_check(this, Shape);
    this.kind = kind;
};
var Box = /*#__PURE__*/ function(Shape) {
    "use strict";
    _inherits(Box, Shape);
    function Box() {
        var width = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : 1, height = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : width * 2, label = arguments.length > 2 ? arguments[2] : void 0;
        _class_call_check(this, Box);
        var _this;
        _this = _call_super(this, Box, [
            "box"
        ]);
        _this.width = width;
        _this.height = height;
        _this.label = label;
        return _this;
    }
    _create_class(Box, [
        {
            key: "area",
            value: function area() {
                var scale = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : 1;
                return this.width * this.height * scale;
            }
        }
    ]);
    return Box;
}(Shape);
var Tagged = /*#__PURE__*/ function(Box) {
    "use strict";
    _inherits(Tagged, Box);
    function Tagged() {
        var tag = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : "t";
        for(var _len = arguments.length, rest = new Array(_len > 1 ? _len - 1 : 0), _key = 1; _key < _len; _key++){
            rest[_key - 1] = arguments[_key];
        }
        _class_call_check(this, Tagged);
        var _this;
        _this = _call_super(this, Tagged, _to_consumable_array(rest));
        _this.tag = tag;
        return _this;
    }
    return Tagged;
}(Box);
function later() {
    var step = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : 1;
    for(var _len = arguments.length, more = new Array(_len > 1 ? _len - 1 : 0), _key = 1; _key < _len; _key++){
        more[_key - 1] = arguments[_key];
    }
    var run = function run() {
        return step + more.length;
    };
    return run();
}
console.log(JSON.stringify(new Box()), JSON.stringify(new Box(3)), JSON.stringify(new Box(3, 4, "x")));
console.log(new Box(2).area(), new Box(2).area(3), Box.length, Box.prototype.area.length);
console.log(JSON.stringify(new Tagged()), JSON.stringify(new Tagged("q", 5)), Tagged.length);
console.log(later(), later(2, 0, 0), later.length);
