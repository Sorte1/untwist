var Counter = /** @class */ (function () {
    function Counter(start) {
        if (start === void 0) { start = 0; }
        this.count = start;
    }
    Counter.prototype.stepper = function (by, label) {
        var _this = this;
        if (by === void 0) { by = 1; }
        if (label === void 0) { label = "n" + this.count; }
        return function () {
            _this.count += by;
            return label + ":" + _this.count;
        };
    };
    return Counter;
}());
function tally(seed, extra) {
    var _this = this;
    if (seed === void 0) { seed = 10; }
    return function () { return _this.base + seed + (extra !== null && extra !== void 0 ? extra : 0) + arguments.length; };
}
var counter = new Counter(2);
var step = counter.stepper();
console.log(step(), step(), counter.stepper(5, "x")(), Counter.prototype.stepper.length);
console.log(tally.call({ base: 1 })(), tally.call({ base: 1 }, 3, 4)(), tally.length);
