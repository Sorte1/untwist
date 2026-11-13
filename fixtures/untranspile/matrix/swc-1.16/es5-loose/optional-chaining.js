"use strict";
function _type_of(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
}
var _ref;
var _data_user, _data_empty, _data_missing, _data_user_tags, _data_user1, _data_empty1, _get_user, _get, _data_user2, _data_empty2, _data_user3;
var data = {
    user: {
        name: "ann",
        tags: [
            "a"
        ],
        greet: function greet() {
            return "hi " + this.name;
        }
    },
    empty: null
};
var calls = 0;
var get = function get() {
    calls++;
    return data;
};
console.log((_data_user = data.user) == null ? void 0 : _data_user.name, (_data_empty = data.empty) == null ? void 0 : _data_empty.name, (_data_missing = data.missing) == null ? void 0 : _data_missing.deep.deeper);
console.log((_data_user1 = data.user) == null ? void 0 : (_data_user_tags = _data_user1.tags) == null ? void 0 : _data_user_tags[0], (_data_empty1 = data.empty) == null ? void 0 : _data_empty1[0], data.user.greet == null ? void 0 : data.user.greet.call(data.user), data.user.nope == null ? void 0 : data.user.nope.call(data.user));
console.log((_get = get()) == null ? void 0 : (_get_user = _get.user) == null ? void 0 : _get_user.name, calls);
var fn = null;
console.log(fn == null ? void 0 : fn(1), _type_of(fn == null ? void 0 : fn.call));
var key = "name";
console.log(data == null ? void 0 : (_data_user2 = data.user) == null ? void 0 : _data_user2[key], (_ref = (_data_empty2 = data.empty) == null ? void 0 : _data_empty2.x) != null ? _ref : "default");
console.log(data == null ? true : (_data_user3 = data.user) == null ? true : delete _data_user3.tags, data.user.tags);
var Node = /*#__PURE__*/ function() {
    "use strict";
    function Node(next) {
        this.next = next;
    }
    var _proto = Node.prototype;
    _proto.depth = function depth() {
        var _ref;
        var _this_next;
        return 1 + ((_ref = (_this_next = this.next) == null ? void 0 : _this_next.depth()) != null ? _ref : 0);
    };
    return Node;
}();
console.log(new Node(new Node(new Node())).depth());
function first(arr) {
    var _arr_;
    return arr == null ? void 0 : (_arr_ = arr[0]) == null ? void 0 : _arr_.value;
}
console.log(first([
    {
        value: 5
    }
]), first(undefined), first([]));
