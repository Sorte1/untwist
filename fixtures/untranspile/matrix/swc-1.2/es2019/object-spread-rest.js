"use strict";
const base = {
    a: 1,
    b: 2
};
const extra = {
    c: 3
};
const merged = {
    ...base,
    ...extra,
    d: 4
};
const overridden = {
    ...base,
    a: 9
};
console.log(JSON.stringify(merged), JSON.stringify(overridden));
const { a , ...others } = merged;
console.log(a, JSON.stringify(others));
function without({ secret , ...pub }) {
    return pub;
}
console.log(JSON.stringify(without({
    secret: 1,
    open: 2
})));
const withGetter = {
    ...{
        get g () {
            return "got";
        }
    }
};
console.log(Object.getOwnPropertyDescriptor(withGetter, "g").value);
const sym = Symbol("s");
const symObj = {
    ...{
        [sym]: 1,
        plain: 2
    }
};
console.log(symObj[sym], symObj.plain);
const { [sym]: picked , ...restSym } = symObj;
console.log(picked, Object.keys(restSym).join(","));
console.log(JSON.stringify({
    ...null,
    ...undefined,
    ..."hi",
    ...[
        5
    ]
}));
let order = [];
const tracked = {
    get x () {
        order.push("x");
        return 1;
    },
    get y () {
        order.push("y");
        return 2;
    }
};
const copy = {
    ...tracked
};
console.log(order.join(","), copy.x + copy.y);
