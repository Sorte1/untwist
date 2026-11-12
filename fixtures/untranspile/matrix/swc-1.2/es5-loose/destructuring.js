"use strict";
function _objectWithoutPropertiesLoose(source, excluded) {
    if (source == null) return {};
    var target = {};
    var sourceKeys = Object.keys(source);
    var key, i;
    for(i = 0; i < sourceKeys.length; i++){
        key = sourceKeys[i];
        if (excluded.indexOf(key) >= 0) continue;
        target[key] = source[key];
    }
    return target;
}
var __generator = (void 0) && (void 0).__generator || function(thisArg, body) {
    var f, y, t, g, _ = {
        label: 0,
        sent: function() {
            if (t[0] & 1) throw t[1];
            return t[1];
        },
        trys: [],
        ops: []
    };
    return g = {
        next: verb(0),
        "throw": verb(1),
        "return": verb(2)
    }, typeof Symbol === "function" && (g[Symbol.iterator] = function() {
        return this;
    }), g;
    function verb(n) {
        return function(v) {
            return step([
                n,
                v
            ]);
        };
    }
    function step(op) {
        if (f) throw new TypeError("Generator is already executing.");
        while(_)try {
            if (f = 1, y && (t = op[0] & 2 ? y["return"] : op[0] ? y["throw"] || ((t = y["return"]) && t.call(y), 0) : y.next) && !(t = t.call(y, op[1])).done) return t;
            if (y = 0, t) op = [
                op[0] & 2,
                t.value
            ];
            switch(op[0]){
                case 0:
                case 1:
                    t = op;
                    break;
                case 4:
                    _.label++;
                    return {
                        value: op[1],
                        done: false
                    };
                case 5:
                    _.label++;
                    y = op[1];
                    op = [
                        0
                    ];
                    continue;
                case 7:
                    op = _.ops.pop();
                    _.trys.pop();
                    continue;
                default:
                    if (!(t = _.trys, t = t.length > 0 && t[t.length - 1]) && (op[0] === 6 || op[0] === 2)) {
                        _ = 0;
                        continue;
                    }
                    if (op[0] === 3 && (!t || op[1] > t[0] && op[1] < t[3])) {
                        _.label = op[1];
                        break;
                    }
                    if (op[0] === 6 && _.label < t[1]) {
                        _.label = t[1];
                        t = op;
                        break;
                    }
                    if (t && _.label < t[2]) {
                        _.label = t[2];
                        _.ops.push(op);
                        break;
                    }
                    if (t[2]) _.ops.pop();
                    _.trys.pop();
                    continue;
            }
            op = body.call(thisArg, _);
        } catch (e) {
            op = [
                6,
                e
            ];
            y = 0;
        } finally{
            f = t = 0;
        }
        if (op[0] & 5) throw op[1];
        return {
            value: op[0] ? op[1] : void 0,
            done: true
        };
    }
};
var ref = [
    1,
    2,
    undefined,
    4,
    5
], a = ref[0], tmp = ref[2], b = tmp === void 0 ? 10 : tmp, rest = ref.slice(3);
var _ref = {
    x: 1,
    q: 9,
    r: 8
}, x = _ref.x, tmp1 = _ref.y, why = tmp1 === void 0 ? 2 : tmp1, tmp2 = _ref.z, deep = (tmp2 === void 0 ? {
    deep: "d"
} : tmp2).deep, others = _objectWithoutPropertiesLoose(_ref, [
    "x",
    "y",
    "z"
]);
console.log(a, b, rest.join(","), x, why, deep, JSON.stringify(others));
function point(param, param1) {
    var ref = param === void 0 ? {} : param, _x = ref.x, x = _x === void 0 ? 0 : _x, _y = ref.y, y = _y === void 0 ? 0 : _y;
    var ref1 = param1 === void 0 ? [] : param1, first = ref1[0], second = ref1[1];
    return x + y + (first || 0) + (second || 0);
}
console.log(point({
    x: 1
}, [
    2,
    3
]), point());
var m = 1;
var n = 2;
var ref1;
ref1 = [
    n,
    m
], m = ref1[0], n = ref1[1], ref1;
console.log(m, n);
var pairs = [
    [
        1,
        "one"
    ],
    [
        2,
        "two"
    ]
];
for(var _i = 0; _i < pairs.length; _i++){
    var __i = pairs[_i], num = __i[0], word = __i[1];
    console.log(num, word);
}
var length = "hello".length;
var ref2 = "hi", c1 = ref2[0], c2 = ref2[1];
console.log(length, c1, c2);
var data = {
    list: [
        {
            id: 7,
            tags: [
                "t1",
                "t2"
            ]
        }
    ]
};
var _list = data.list, ref3 = _list[0], id = ref3.id, _tags = ref3.tags, second = _tags[1];
console.log(id, second);
var key = "dyn";
var ref4 = {}, tmp3 = ref4[key], dynamic = tmp3 === void 0 ? "fallback" : tmp3;
console.log(dynamic);
function swap(param) {
    var p = param[0], q = param[1];
    return [
        q,
        p
    ];
}
console.log(swap([
    1,
    2
]).join(","));
var ref5 = function() {
    return __generator(this, function(_state) {
        switch(_state.label){
            case 0:
                return [
                    4,
                    "a"
                ];
            case 1:
                _state.sent();
                return [
                    4,
                    "b"
                ];
            case 2:
                _state.sent();
                return [
                    4,
                    "c"
                ];
            case 3:
                _state.sent();
                return [
                    2
                ];
        }
    });
}(), g1 = ref5[0], g2 = ref5[1];
console.log(g1, g2);
