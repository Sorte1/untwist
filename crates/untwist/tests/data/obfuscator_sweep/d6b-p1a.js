function g(x) { return x + 1; }
function f(x) {
    var C = (function () {
        var first = !![];
        return function (ctx, fn) {
            var r = first ? function () {
                if (fn) {
                    var x = fn['apply'](ctx, arguments);
                    fn = null;
                    return x;
                }
            } : function () {
            };
            first = ![];
            return r;
        };
    }());
    C(this, function () { globalThis.console.log("LIVE"); })();
    return g(x);
}
console.log(f(1));
console.log(f(2));
