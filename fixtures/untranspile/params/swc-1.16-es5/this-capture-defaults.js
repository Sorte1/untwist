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
var Counter = /*#__PURE__*/ function() {
    "use strict";
    function Counter() {
        var start = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : 0;
        _class_call_check(this, Counter);
        this.count = start;
    }
    _create_class(Counter, [
        {
            key: "stepper",
            value: function stepper() {
                var _this = this;
                var by = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : 1, label = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : "n" + this.count;
                return function() {
                    _this.count += by;
                    return label + ":" + _this.count;
                };
            }
        }
    ]);
    return Counter;
}();
function tally() {
    var _this = this, _arguments = arguments;
    var seed = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : 10, extra = arguments.length > 1 ? arguments[1] : void 0;
    return function() {
        return _this.base + seed + (extra !== null && extra !== void 0 ? extra : 0) + _arguments.length;
    };
}
var counter = new Counter(2);
var step = counter.stepper();
console.log(step(), step(), counter.stepper(5, "x")(), Counter.prototype.stepper.length);
console.log(tally.call({
    base: 1
})(), tally.call({
    base: 1
}, 3, 4)(), tally.length);
