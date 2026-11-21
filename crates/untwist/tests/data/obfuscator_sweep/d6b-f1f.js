var C = (function () {
    var first = true;
    return function (ctx, fn) {
        var r = first ? function () {
            if (fn) {
                var x = fn.apply(ctx, arguments);
                fn = null;
                return x;
            }
        } : function () {
        };
        first = false;
        return r;
    };
}());
function g(x) { return x + 1; }
function f(x) {
    C(this, function () { })();
    return g(x);
}
console.log(f(1));
console.log(C(this, function () { return 5; })());
