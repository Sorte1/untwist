(function () {
    'use strict';

    var _a, _b, _c, _d, _e, _f, _g, _h, _j, _k, _l, _m, _o, _p, _q, _r;
    var data = { user: { name: "ann", tags: ["a"], greet: function () { return "hi " + this.name; } }, empty: null };
    var calls = 0;
    var get = function () {
        calls++;
        return data;
    };
    console.log((_a = data.user) === null || _a === void 0 ? void 0 : _a.name, (_b = data.empty) === null || _b === void 0 ? void 0 : _b.name, (_c = data.missing) === null || _c === void 0 ? void 0 : _c.deep.deeper);
    console.log((_e = (_d = data.user) === null || _d === void 0 ? void 0 : _d.tags) === null || _e === void 0 ? void 0 : _e[0], (_f = data.empty) === null || _f === void 0 ? void 0 : _f[0], (_h = (_g = data.user).greet) === null || _h === void 0 ? void 0 : _h.call(_g), (_k = (_j = data.user).nope) === null || _k === void 0 ? void 0 : _k.call(_j));
    console.log((_m = (_l = get()) === null || _l === void 0 ? void 0 : _l.user) === null || _m === void 0 ? void 0 : _m.name, calls);
    console.log(void 0 , "undefined");
    var key = "name";
    console.log((_o = data === null || data === void 0 ? void 0 : data.user) === null || _o === void 0 ? void 0 : _o[key], (_q = (_p = (data.empty)) === null || _p === void 0 ? void 0 : _p.x) !== null && _q !== void 0 ? _q : "default");
    console.log((_r = data === null || data === void 0 ? void 0 : data.user) === null || _r === void 0 ? true : delete _r.tags, data.user.tags);
    var Node = /** @class */ (function () {
        function Node(next) {
            this.next = next;
        }
        Node.prototype.depth = function () {
            var _a, _b;
            return 1 + ((_b = (_a = this.next) === null || _a === void 0 ? void 0 : _a.depth()) !== null && _b !== void 0 ? _b : 0);
        };
        return Node;
    }());
    console.log(new Node(new Node(new Node())).depth());
    function first(arr) {
        var _a;
        return (_a = arr === null || arr === void 0 ? void 0 : arr[0]) === null || _a === void 0 ? void 0 : _a.value;
    }
    console.log(first([{ value: 5 }]), first(undefined), first([]));

})();
