"use strict";
function _assert_this_initialized(self) {
    if (self === void 0) {
        throw new ReferenceError("this hasn't been initialised - super() hasn't been called");
    }
    return self;
}
function _class_call_check(instance, Constructor) {
    if (!(instance instanceof Constructor)) {
        throw new TypeError("Cannot call a class as a function");
    }
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
function _possible_constructor_return(self, call) {
    if (call && (_type_of(call) === "object" || typeof call === "function")) {
        return call;
    }
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
function _create_super(Derived) {
    var hasNativeReflectConstruct = _is_native_reflect_construct();
    return function _createSuperInternal() {
        var Super = _get_prototype_of(Derived), result;
        if (hasNativeReflectConstruct) {
            var NewTarget = _get_prototype_of(this).constructor;
            result = Reflect.construct(Super, arguments, NewTarget);
        } else {
            result = Super.apply(this, arguments);
        }
        return _possible_constructor_return(this, result);
    };
}
var Shape = /*#__PURE__*/ function() {
    "use strict";
    function Shape(kind) {
        _class_call_check(this, Shape);
        this.kind = kind;
    }
    _create_class(Shape, [
        {
            key: "area",
            value: function area() {
                return 0;
            }
        }
    ], [
        {
            key: "of",
            value: function of(kind) {
                return new Shape(kind);
            }
        }
    ]);
    return Shape;
}();
var Square = /*#__PURE__*/ function(Shape) {
    "use strict";
    _inherits(Square, Shape);
    var _super = _create_super(Square);
    function Square(side) {
        _class_call_check(this, Square);
        var _this;
        _this = _super.call(this, "square");
        _this.side = side;
        return _this;
    }
    _create_class(Square, [
        {
            key: "area",
            value: function area() {
                return this.side * this.side;
            }
        }
    ]);
    return Square;
}(Shape);
var s = new Square(3);
var seen = [];
for(var key in s)seen.push(key);
console.log(seen.join(","), Object.keys(Shape.prototype).length, Object.getOwnPropertyNames(Square.prototype).join(","));
console.log(Object.getOwnPropertyDescriptor(Shape, "prototype").writable, Object.getOwnPropertyDescriptor(Shape.prototype, "area").enumerable);
for(var _i = 0, _iter = [
    function() {
        return Shape("x");
    },
    function() {
        return Square(1);
    }
]; _i < _iter.length; _i++){
    var make = _iter[_i];
    try {
        make();
        console.log("called without new");
    } catch (e) {
        console.log(e.constructor.name);
    }
}
console.log(Shape.length, Square.length, _type_of(Shape.prototype.area.prototype));
