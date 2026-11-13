"use strict";
function _array_like_to_array(arr, len) {
    if (len == null || len > arr.length) len = arr.length;
    for(var i = 0, arr2 = new Array(len); i < len; i++)arr2[i] = arr[i];
    return arr2;
}
function _array_with_holes(arr) {
    if (Array.isArray(arr)) return arr;
}
function _iterable_to_array_limit(arr, i) {
    var _i = arr == null ? null : typeof Symbol !== "undefined" && arr[Symbol.iterator] || arr["@@iterator"];
    if (_i == null) return;
    var _arr = [];
    var _n = true;
    var _d = false;
    var _s, _e;
    try {
        for(_i = _i.call(arr); !(_n = (_s = _i.next()).done); _n = true){
            _arr.push(_s.value);
            if (i && _arr.length === i) break;
        }
    } catch (err) {
        _d = true;
        _e = err;
    } finally{
        try {
            if (!_n && _i["return"] != null) _i["return"]();
        } finally{
            if (_d) throw _e;
        }
    }
    return _arr;
}
function _non_iterable_rest() {
    throw new TypeError("Invalid attempt to destructure non-iterable instance.\\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
}
function _object_without_properties(source, excluded) {
    if (source == null) return {};
    var target = _object_without_properties_loose(source, excluded);
    var key, i;
    if (Object.getOwnPropertySymbols) {
        var sourceSymbolKeys = Object.getOwnPropertySymbols(source);
        for(i = 0; i < sourceSymbolKeys.length; i++){
            key = sourceSymbolKeys[i];
            if (excluded.indexOf(key) >= 0) continue;
            if (!Object.prototype.propertyIsEnumerable.call(source, key)) continue;
            target[key] = source[key];
        }
    }
    return target;
}
function _object_without_properties_loose(source, excluded) {
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
function _sliced_to_array(arr, i) {
    return _array_with_holes(arr) || _iterable_to_array_limit(arr, i) || _unsupported_iterable_to_array(arr, i) || _non_iterable_rest();
}
function _unsupported_iterable_to_array(o, minLen) {
    if (!o) return;
    if (typeof o === "string") return _array_like_to_array(o, minLen);
    var n = Object.prototype.toString.call(o).slice(8, -1);
    if (n === "Object" && o.constructor) n = o.constructor.name;
    if (n === "Map" || n === "Set") return Array.from(n);
    if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return _array_like_to_array(o, minLen);
}
function _ts_generator(thisArg, body) {
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
}
var _ref = [
    1,
    2,
    undefined,
    4,
    5
], a = _ref[0], tmp = _ref[2], b = tmp === void 0 ? 10 : tmp, rest = _ref.slice(3);
var _ref1 = {
    x: 1,
    q: 9,
    r: 8
}, x = _ref1.x, tmp1 = _ref1.y, why = tmp1 === void 0 ? 2 : tmp1, tmp2 = _ref1.z, deep = (tmp2 === void 0 ? {
    deep: "d"
} : tmp2).deep, others = _object_without_properties(_ref1, [
    "x",
    "y",
    "z"
]);
console.log(a, b, rest.join(","), x, why, deep, JSON.stringify(others));
function point() {
    var _ref = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : {}, _ref_x = _ref.x, x = _ref_x === void 0 ? 0 : _ref_x, _ref_y = _ref.y, y = _ref_y === void 0 ? 0 : _ref_y, _ref1 = _sliced_to_array(arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : [], 2), first = _ref1[0], second = _ref1[1];
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
var ref;
ref = [
    n,
    m
], m = ref[0], n = ref[1], ref;
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
var _iteratorNormalCompletion = true, _didIteratorError = false, _iteratorError = undefined;
try {
    for(var _iterator = pairs[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true){
        var _step_value = _sliced_to_array(_step.value, 2), num = _step_value[0], word = _step_value[1];
        console.log(num, word);
    }
} catch (err) {
    _didIteratorError = true;
    _iteratorError = err;
} finally{
    try {
        if (!_iteratorNormalCompletion && _iterator.return != null) {
            _iterator.return();
        }
    } finally{
        if (_didIteratorError) {
            throw _iteratorError;
        }
    }
}
var length = "hello".length;
var _hi = _sliced_to_array("hi", 2), c1 = _hi[0], c2 = _hi[1];
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
var _data_list = _sliced_to_array(data.list, 1), _data_list_ = _data_list[0], id = _data_list_.id, _data_list__tags = _sliced_to_array(_data_list_.tags, 2), second = _data_list__tags[1];
console.log(id, second);
var key = "dyn";
var _ref2 = {}, tmp3 = _ref2[key], dynamic = tmp3 === void 0 ? "fallback" : tmp3;
console.log(dynamic);
function swap(param) {
    var _param = _sliced_to_array(param, 2), p = _param[0], q = _param[1];
    return [
        q,
        p
    ];
}
console.log(swap([
    1,
    2
]).join(","));
var _ref3 = _sliced_to_array(function() {
    return _ts_generator(this, function(_state) {
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
}(), 2), g1 = _ref3[0], g2 = _ref3[1];
console.log(g1, g2);
