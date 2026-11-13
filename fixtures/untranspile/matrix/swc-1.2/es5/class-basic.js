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
var _typeof = function(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
};
var Point = /*#__PURE__*/ function() {
    "use strict";
    function Point(x, y) {
        _classCallCheck(this, Point);
        this.x = x;
        this.y = y;
    }
    _createClass(Point, [
        {
            key: "length",
            get: function get() {
                return Math.sqrt(this.x * this.x + this.y * this.y);
            },
            set: function set(v) {
                var k = v / this.length;
                this.x *= k;
                this.y *= k;
            }
        },
        {
            key: "add",
            value: function add(other) {
                return new Point(this.x + other.x, this.y + other.y);
            }
        },
        {
            key: "to" + "Pair",
            value: function value() {
                return [
                    this.x,
                    this.y
                ];
            }
        }
    ], [
        {
            key: "origin",
            value: function origin() {
                return new this(0, 0);
            }
        },
        {
            key: "unit",
            get: function get() {
                return new Point(1, 0);
            }
        }
    ]);
    return Point;
}();
var p = new Point(3, 4);
console.log(p.length, p.add(Point.origin()).toPair().join(","), Point.unit.x);
p.length = 10;
console.log(p.x, p.y);
console.log(typeof Point === "undefined" ? "undefined" : _typeof(Point), Point.name);
var Anon = /*#__PURE__*/ function() {
    "use strict";
    function Anon() {
        _classCallCheck(this, Anon);
    }
    _createClass(Anon, [
        {
            key: "hi",
            value: function hi() {
                return "anon";
            }
        }
    ]);
    return Anon;
}();
console.log(new Anon().hi(), Anon.name);
