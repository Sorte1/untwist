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
for(var _i = 0, _iter = new Set(handlers); _i < _iter.length; _i++){
    var handler = _iter[_i];
    handler();
}
var counts = {};
for(var _i1 = 0, _iter1 = new Set([
    "x",
    "y",
    "x"
]); _i1 < _iter1.length; _i1++){
    var key = _iter1[_i1];
    counts[key] = (counts[key] || 0) + 1;
}
for(var _i2 = 0, _iter2 = new Set([
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
]); _i2 < _iter2.length; _i2++){
    var entry = _iter2[_i2];
    for(var _i3 = 0, _list = entry.list; _i3 < _list.length; _i3++){
        var n = _list[_i3];
        calls.push(n);
    }
}
var removed = {
    a: 1,
    b: 2,
    c: 3
};
for(var _i4 = 0, _iter3 = new Set([
    "a",
    "c"
]); _i4 < _iter3.length; _i4++){
    var key1 = _iter3[_i4];
    delete removed[key1];
}
var total = 0;
var weights = {
    p: 2,
    q: 5
};
for(var _i5 = 0, _iter4 = new Set([
    "p",
    "q",
    "r"
]); _i5 < _iter4.length; _i5++){
    var name = _iter4[_i5];
    total += weights[name] || 0;
}
for(var _i6 = 0, _iter5 = new Set([
    {
        run: function(k) {
            return calls.push(k);
        }
    }
]); _i6 < _iter5.length; _i6++){
    var item = _iter5[_i6];
    item.run("ran");
}
console.log(calls.join(), JSON.stringify(counts), JSON.stringify(removed), total);
