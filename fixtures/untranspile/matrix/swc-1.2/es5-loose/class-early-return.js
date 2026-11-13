"use strict";
function _assertThisInitialized(self) {
    if (self === void 0) {
        throw new ReferenceError("this hasn't been initialised - super() hasn't been called");
    }
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
    if (superClass) _setPrototypeOf(subClass, superClass);
}
function _instanceof(left, right) {
    if (right != null && typeof Symbol !== "undefined" && right[Symbol.hasInstance]) {
        return !!right[Symbol.hasInstance](left);
    } else {
        return left instanceof right;
    }
}
function _possibleConstructorReturn(self, call) {
    if (call && (_typeof(call) === "object" || typeof call === "function")) {
        return call;
    }
    return _assertThisInitialized(self);
}
function _setPrototypeOf(o, p) {
    _setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) {
        o.__proto__ = p;
        return o;
    };
    return _setPrototypeOf(o, p);
}
var _typeof = function(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
};
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
            return _possibleConstructorReturn(_this);
        }
        _this.items = [];
        return _this;
    }
    return Pooled;
}(Resource);
var ref = [
    new Pooled("a"),
    new Pooled("b", 16)
], first = ref[0], second = ref[1];
console.log(first.items.length, second.capped, second.items, _instanceof(first, Resource));
