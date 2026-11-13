"use strict";
function _classCallCheck(instance, Constructor) {
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
function _createClass(Constructor, protoProps, staticProps) {
    if (protoProps) _defineProperties(Constructor.prototype, protoProps);
    if (staticProps) _defineProperties(Constructor, staticProps);
    return Constructor;
}
function _instanceof(left, right) {
    if (right != null && typeof Symbol !== "undefined" && right[Symbol.hasInstance]) {
        return !!right[Symbol.hasInstance](left);
    } else {
        return left instanceof right;
    }
}
var _typeof = function(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
};
var counter = {
    count: 0,
    start: function start() {
        var _this = this;
        [
            1,
            2,
            3
        ].forEach(function(n) {
            _this.count += n;
        });
        return this.count;
    },
    nested: function nested() {
        var _this = this;
        var outer = function() {
            var inner = function() {
                return _this.count;
            };
            return inner();
        };
        return outer();
    },
    argsLater: function argsLater() {
        var _arguments = arguments;
        var f = function() {
            return _arguments[0] + _arguments.length;
        };
        return f("x", "y");
    }
};
console.log(counter.start(), counter.nested(), counter.argsLater("a", "b"));
function Timer() {
    var _this = this;
    this.ticks = 0;
    var tick = function() {
        return ++_this.ticks;
    };
    tick();
    tick();
}
console.log(new Timer().ticks);
var Widget = /*#__PURE__*/ function() {
    "use strict";
    function Widget() {
        var _this = this;
        _classCallCheck(this, Widget);
        this.name = "w";
        this.handler = function() {
            return _this.name;
        };
    }
    _createClass(Widget, [
        {
            key: "render",
            value: function render() {
                var _this = this;
                return [
                    1
                ].map(function() {
                    return _this.name + _this.handler();
                })[0];
            }
        }
    ]);
    return Widget;
}();
console.log(new Widget().render());
var id = function(x) {
    return x;
};
var obj = function(a, b) {
    return {
        a: a,
        b: b
    };
};
console.log(id(5), JSON.stringify(obj(1, 2)));
function outerFn() {
    var _newtarget = _instanceof(this, outerFn) ? this.constructor : void 0;
    return function() {
        return typeof _newtarget === "undefined" ? "undefined" : _typeof(_newtarget);
    };
}
console.log(outerFn()(), new outerFn()());
