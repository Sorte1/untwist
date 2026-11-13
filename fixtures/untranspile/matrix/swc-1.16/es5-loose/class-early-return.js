"use strict";
function _assert_this_initialized(self) {
    if (self === void 0) throw new ReferenceError("this hasn't been initialised - super() hasn't been called");
    return self;
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
var Resource = function Resource(name) {
    "use strict";
    this.name = name;
};
var Pooled = /*#__PURE__*/ function(Resource) {
    "use strict";
    _inherits(Pooled, Resource);
    function Pooled(name, size) {
        if (size === void 0) size = 4;
        var _this;
        _this = Resource.call(this, name) || this;
        _this.size = size;
        if (size > 8) {
            _this.capped = true;
            return _possible_constructor_return(_this);
        }
        _this.items = [];
        return _this;
    }
    return Pooled;
}(Resource);
var _ref = [
    new Pooled("a"),
    new Pooled("b", 16)
], first = _ref[0], second = _ref[1];
console.log(first.items.length, second.capped, second.items, _instanceof(first, Resource));
