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
var K = (function () {
    C(this, function () { })();
    var once = true;
    return function (ctx, fn) {
        var r = once ? function () {
            if (fn) {
                var x = fn.apply(ctx, arguments);
                fn = null;
                return x;
            }
        } : function () {
        };
        once = false;
        return r;
    };
}());
console.log(C(this, function () { return 5; })());
console.log(K(this, function () { return 6; })());
