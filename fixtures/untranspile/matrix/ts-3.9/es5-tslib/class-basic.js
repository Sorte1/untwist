(function () {
    'use strict';

    var Point = /** @class */ (function () {
        function Point(x, y) {
            this.x = x;
            this.y = y;
        }
        Object.defineProperty(Point.prototype, "length", {
            get: function () {
                return Math.sqrt(this.x * this.x + this.y * this.y);
            },
            set: function (v) {
                var k = v / this.length;
                this.x *= k;
                this.y *= k;
            },
            enumerable: false,
            configurable: true
        });
        Point.prototype.add = function (other) {
            return new Point(this.x + other.x, this.y + other.y);
        };
        Point.prototype["to" + "Pair"] = function () {
            return [this.x, this.y];
        };
        Point.origin = function () {
            return new this(0, 0);
        };
        Object.defineProperty(Point, "unit", {
            get: function () {
                return new Point(1, 0);
            },
            enumerable: false,
            configurable: true
        });
        return Point;
    }());
    var p = new Point(3, 4);
    console.log(p.length, p.add(Point.origin()).toPair().join(","), Point.unit.x);
    p.length = 10;
    console.log(p.x, p.y);
    console.log(typeof Point, Point.name);
    var Anon = /** @class */ (function () {
        function Anon() {
        }
        Anon.prototype.hi = function () {
            return "anon";
        };
        return Anon;
    }());
    console.log(new Anon().hi(), Anon.name);

})();
