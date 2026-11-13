(function () {
    'use strict';

    var _a, _b, _c, _d, _e, _f, _g;
    function pick(a, b) {
        return a !== null && a !== void 0 ? a : b;
    }
    console.log(pick(0, 1), pick("", "x"), pick(null, "n"), pick(undefined, "u"), pick(false, true));
    var cfg = { retries: 0, name: "" };
    var retries = (_a = cfg.retries) !== null && _a !== void 0 ? _a : 3;
    var name = (_b = cfg.name) !== null && _b !== void 0 ? _b : "anon";
    var missing = (_c = cfg.timeout) !== null && _c !== void 0 ? _c : 1000;
    console.log(retries, name, missing);
    var calls = 0;
    var side = function () {
        calls++;
        return null;
    };
    console.log((_e = (_d = side()) !== null && _d !== void 0 ? _d : side()) !== null && _e !== void 0 ? _e : "end", calls);
    console.log(((_f = null) !== null && _f !== void 0 ? _f : 0) || "or", (_g = null) !== null && _g !== void 0 ? _g : ("inner"));

})();
