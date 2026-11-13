"use strict";
var calls = [];
var handlers = [
    function() {
        return calls.push("a");
    },
    function() {
        return calls.push("b");
    }
];
var _iteratorNormalCompletion = true, _didIteratorError = false, _iteratorError = undefined;
try {
    for(var _iterator = new Set(handlers)[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true){
        var handler = _step.value;
        handler();
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
var counts = {};
var _iteratorNormalCompletion1 = true, _didIteratorError1 = false, _iteratorError1 = undefined;
try {
    for(var _iterator1 = new Set([
        "x",
        "y",
        "x"
    ])[Symbol.iterator](), _step1; !(_iteratorNormalCompletion1 = (_step1 = _iterator1.next()).done); _iteratorNormalCompletion1 = true){
        var key = _step1.value;
        counts[key] = (counts[key] || 0) + 1;
    }
} catch (err) {
    _didIteratorError1 = true;
    _iteratorError1 = err;
} finally{
    try {
        if (!_iteratorNormalCompletion1 && _iterator1.return != null) {
            _iterator1.return();
        }
    } finally{
        if (_didIteratorError1) {
            throw _iteratorError1;
        }
    }
}
var _iteratorNormalCompletion2 = true, _didIteratorError2 = false, _iteratorError2 = undefined, _iteratorNormalCompletion3 = true, _didIteratorError3 = false, _iteratorError3 = undefined;
try {
    for(var _iterator2 = new Set([
        {
            list: [
                1,
                2
            ]
        },
        {
            list: [
                3
            ]
        }
    ])[Symbol.iterator](), _step2; !(_iteratorNormalCompletion3 = (_step2 = _iterator2.next()).done); _iteratorNormalCompletion3 = true){
        var entry = _step2.value;
        try {
            for(var _iterator3 = entry.list[Symbol.iterator](), _step3; !(_iteratorNormalCompletion2 = (_step3 = _iterator3.next()).done); _iteratorNormalCompletion2 = true){
                var n = _step3.value;
                calls.push(n);
            }
        } catch (err) {
            _didIteratorError2 = true;
            _iteratorError2 = err;
        } finally{
            try {
                if (!_iteratorNormalCompletion2 && _iterator3.return != null) {
                    _iterator3.return();
                }
            } finally{
                if (_didIteratorError2) {
                    throw _iteratorError2;
                }
            }
        }
    }
} catch (err) {
    _didIteratorError3 = true;
    _iteratorError3 = err;
} finally{
    try {
        if (!_iteratorNormalCompletion3 && _iterator2.return != null) {
            _iterator2.return();
        }
    } finally{
        if (_didIteratorError3) {
            throw _iteratorError3;
        }
    }
}
var removed = {
    a: 1,
    b: 2,
    c: 3
};
var _iteratorNormalCompletion4 = true, _didIteratorError4 = false, _iteratorError4 = undefined;
try {
    for(var _iterator4 = new Set([
        "a",
        "c"
    ])[Symbol.iterator](), _step4; !(_iteratorNormalCompletion4 = (_step4 = _iterator4.next()).done); _iteratorNormalCompletion4 = true){
        var key1 = _step4.value;
        delete removed[key1];
    }
} catch (err) {
    _didIteratorError4 = true;
    _iteratorError4 = err;
} finally{
    try {
        if (!_iteratorNormalCompletion4 && _iterator4.return != null) {
            _iterator4.return();
        }
    } finally{
        if (_didIteratorError4) {
            throw _iteratorError4;
        }
    }
}
var total = 0;
var weights = {
    p: 2,
    q: 5
};
var _iteratorNormalCompletion5 = true, _didIteratorError5 = false, _iteratorError5 = undefined;
try {
    for(var _iterator5 = new Set([
        "p",
        "q",
        "r"
    ])[Symbol.iterator](), _step5; !(_iteratorNormalCompletion5 = (_step5 = _iterator5.next()).done); _iteratorNormalCompletion5 = true){
        var name = _step5.value;
        total += weights[name] || 0;
    }
} catch (err) {
    _didIteratorError5 = true;
    _iteratorError5 = err;
} finally{
    try {
        if (!_iteratorNormalCompletion5 && _iterator5.return != null) {
            _iterator5.return();
        }
    } finally{
        if (_didIteratorError5) {
            throw _iteratorError5;
        }
    }
}
var _iteratorNormalCompletion6 = true, _didIteratorError6 = false, _iteratorError6 = undefined;
try {
    for(var _iterator6 = new Set([
        {
            run: function(k) {
                return calls.push(k);
            }
        }
    ])[Symbol.iterator](), _step6; !(_iteratorNormalCompletion6 = (_step6 = _iterator6.next()).done); _iteratorNormalCompletion6 = true){
        var item = _step6.value;
        item.run("ran");
    }
} catch (err) {
    _didIteratorError6 = true;
    _iteratorError6 = err;
} finally{
    try {
        if (!_iteratorNormalCompletion6 && _iterator6.return != null) {
            _iterator6.return();
        }
    } finally{
        if (_didIteratorError6) {
            throw _iteratorError6;
        }
    }
}
console.log(calls.join(), JSON.stringify(counts), JSON.stringify(removed), total);
