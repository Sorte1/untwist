"use strict";
var counter = {
    count: 0,
    start: function () {
        var _this = this;
        [1, 2, 3].forEach(function (n) {
            _this.count += n;
        });
        return this.count;
    },
    nested: function () {
        var _this = this;
        var outer = function () {
            var inner = function () { return _this.count; };
            return inner();
        };
        return outer();
    },
    argsLater: function () {
        var f = function () { return arguments[0] + arguments.length; };
        return f("x", "y");
    },
};
console.log(counter.start(), counter.nested(), counter.argsLater("a", "b"));
function Timer() {
    var _this = this;
    this.ticks = 0;
    var tick = function () { return ++_this.ticks; };
    tick();
    tick();
}
console.log(new Timer().ticks);
var Widget = /** @class */ (function () {
    function Widget() {
        var _this = this;
        this.name = "w";
        this.handler = function () { return _this.name; };
    }
    Widget.prototype.render = function () {
        var _this = this;
        return [1].map(function () { return _this.name + _this.handler(); })[0];
    };
    return Widget;
}());
console.log(new Widget().render());
var id = function (x) { return x; };
var obj = function (a, b) { return ({ a: a, b: b }); };
console.log(id(5), JSON.stringify(obj(1, 2)));
function outerFn() {
    var _newTarget = this && this instanceof outerFn ? this.constructor : void 0;
    return function () { return typeof _newTarget; };
}
console.log(outerFn()(), new outerFn()());
