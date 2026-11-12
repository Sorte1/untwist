"use strict";
var _data_user, _data_empty, _data_missing, _data_user_tags, _data_user1, _data_empty1, _data_user_greet, _data_user2, _data_user_nope, _data_user3, _get_user, _get, _data_user4, _data_empty2, _data_user5;
const data = {
    user: {
        name: "ann",
        tags: [
            "a"
        ],
        greet () {
            return "hi " + this.name;
        }
    },
    empty: null
};
let calls = 0;
const get = ()=>{
    calls++;
    return data;
};
console.log((_data_user = data.user) === null || _data_user === void 0 ? void 0 : _data_user.name, (_data_empty = data.empty) === null || _data_empty === void 0 ? void 0 : _data_empty.name, (_data_missing = data.missing) === null || _data_missing === void 0 ? void 0 : _data_missing.deep.deeper);
console.log((_data_user1 = data.user) === null || _data_user1 === void 0 ? void 0 : (_data_user_tags = _data_user1.tags) === null || _data_user_tags === void 0 ? void 0 : _data_user_tags[0], (_data_empty1 = data.empty) === null || _data_empty1 === void 0 ? void 0 : _data_empty1[0], (_data_user_greet = (_data_user2 = data.user).greet) === null || _data_user_greet === void 0 ? void 0 : _data_user_greet.call(_data_user2), (_data_user_nope = (_data_user3 = data.user).nope) === null || _data_user_nope === void 0 ? void 0 : _data_user_nope.call(_data_user3));
console.log((_get = get()) === null || _get === void 0 ? void 0 : (_get_user = _get.user) === null || _get_user === void 0 ? void 0 : _get_user.name, calls);
const fn = null;
console.log(fn === null || fn === void 0 ? void 0 : fn(1), typeof (fn === null || fn === void 0 ? void 0 : fn.call));
const key = "name";
var _data_empty_x;
console.log(data === null || data === void 0 ? void 0 : (_data_user4 = data.user) === null || _data_user4 === void 0 ? void 0 : _data_user4[key], (_data_empty_x = (_data_empty2 = data.empty) === null || _data_empty2 === void 0 ? void 0 : _data_empty2.x) !== null && _data_empty_x !== void 0 ? _data_empty_x : "default");
console.log(data === null || data === void 0 ? true : (_data_user5 = data.user) === null || _data_user5 === void 0 ? true : delete _data_user5.tags, data.user.tags);
class Node {
    depth() {
        var _this_next;
        var _this_next_depth;
        return 1 + ((_this_next_depth = (_this_next = this.next) === null || _this_next === void 0 ? void 0 : _this_next.depth()) !== null && _this_next_depth !== void 0 ? _this_next_depth : 0);
    }
    constructor(next){
        this.next = next;
    }
}
console.log(new Node(new Node(new Node())).depth());
function first(arr) {
    var _arr_;
    return arr === null || arr === void 0 ? void 0 : (_arr_ = arr[0]) === null || _arr_ === void 0 ? void 0 : _arr_.value;
}
console.log(first([
    {
        value: 5
    }
]), first(undefined), first([]));
