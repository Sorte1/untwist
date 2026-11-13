"use strict";
var _typeof = function(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
};
var ref, ref1, ref2, ref3, ref4, ref5, ref6, ref7, ref8, ref9, ref10;
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
var get = function() {
    calls++;
    return data;
};
console.log((ref = data.user) == null ? void 0 : ref.name, (ref1 = data.empty) == null ? void 0 : ref1.name, (ref2 = data.missing) == null ? void 0 : ref2.deep.deeper);
console.log((ref3 = data.user) == null ? void 0 : (ref4 = ref3.tags) == null ? void 0 : ref4[0], (ref5 = data.empty) == null ? void 0 : ref5[0], data.user.greet == null ? void 0 : data.user.greet(), data.user.nope == null ? void 0 : data.user.nope());
console.log((ref6 = get()) == null ? void 0 : (ref7 = ref6.user) == null ? void 0 : ref7.name, calls);
var fn = null;
console.log(fn == null ? void 0 : fn(1), _typeof(fn == null ? void 0 : fn.call));
var key = "name";
var ref11;
console.log(data == null ? void 0 : (ref8 = data.user) == null ? void 0 : ref8[key], (ref11 = (ref9 = data.empty) == null ? void 0 : ref9.x) != null ? ref11 : "default");
console.log(data == null ? void 0 : (ref10 = data.user) == null ? void 0 : delete ref10.tags, data.user.tags);
var Node = /*#__PURE__*/ function() {
    "use strict";
    function Node(next) {
        this.next = next;
    }
    var _proto = Node.prototype;
    _proto.depth = function depth() {
        var ref;
        var ref1;
        return 1 + ((ref1 = (ref = this.next) == null ? void 0 : ref.depth()) != null ? ref1 : 0);
    };
    return Node;
}();
console.log(new Node(new Node(new Node())).depth());
function first(arr) {
    var ref;
    return arr == null ? void 0 : (ref = arr[0]) == null ? void 0 : ref.value;
}
console.log(first([
    {
        value: 5
    }
]), first(undefined), first([]));
