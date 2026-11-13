"use strict";
function _classCallCheck(instance, Constructor) {
    if (!(instance instanceof Constructor)) {
        throw new TypeError("Cannot call a class as a function");
    }
}
function _defineProperties(target, props) {
    for(var i = 0; i < props.length; i++){
        var descriptor = props[i];
        descriptor.enumerable = descriptor.enumerable || false;
        descriptor.configurable = true;
        if ("value" in descriptor) descriptor.writable = true;
        Object.defineProperty(target, descriptor.key, descriptor);
    }
}
function _createClass(Constructor, protoProps, staticProps) {
    if (protoProps) _defineProperties(Constructor.prototype, protoProps);
    if (staticProps) _defineProperties(Constructor, staticProps);
    return Constructor;
}
var _typeof = function(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
};
var ref, ref1, ref2, ref3, ref4, ref5, _user, ref6, _user1, ref7, ref8, ref9, ref10, ref11, ref12;
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
console.log((ref = data.user) === null || ref === void 0 ? void 0 : ref.name, (ref1 = data.empty) === null || ref1 === void 0 ? void 0 : ref1.name, (ref2 = data.missing) === null || ref2 === void 0 ? void 0 : ref2.deep.deeper);
console.log((ref3 = data.user) === null || ref3 === void 0 ? void 0 : (ref4 = ref3.tags) === null || ref4 === void 0 ? void 0 : ref4[0], (ref5 = data.empty) === null || ref5 === void 0 ? void 0 : ref5[0], (ref6 = (_user = data.user).greet) === null || ref6 === void 0 ? void 0 : ref6.call(_user), (ref7 = (_user1 = data.user).nope) === null || ref7 === void 0 ? void 0 : ref7.call(_user1));
console.log((ref8 = get()) === null || ref8 === void 0 ? void 0 : (ref9 = ref8.user) === null || ref9 === void 0 ? void 0 : ref9.name, calls);
var fn = null;
console.log(fn === null || fn === void 0 ? void 0 : fn(1), _typeof(fn === null || fn === void 0 ? void 0 : fn.call));
var key = "name";
var ref13;
console.log(data === null || data === void 0 ? void 0 : (ref10 = data.user) === null || ref10 === void 0 ? void 0 : ref10[key], (ref13 = (ref11 = data.empty) === null || ref11 === void 0 ? void 0 : ref11.x) !== null && ref13 !== void 0 ? ref13 : "default");
console.log(data === null || data === void 0 ? void 0 : (ref12 = data.user) === null || ref12 === void 0 ? void 0 : delete ref12.tags, data.user.tags);
var Node = /*#__PURE__*/ function() {
    "use strict";
    function Node(next) {
        _classCallCheck(this, Node);
        this.next = next;
    }
    _createClass(Node, [
        {
            key: "depth",
            value: function depth() {
                var ref;
                var ref1;
                return 1 + ((ref1 = (ref = this.next) === null || ref === void 0 ? void 0 : ref.depth()) !== null && ref1 !== void 0 ? ref1 : 0);
            }
        }
    ]);
    return Node;
}();
console.log(new Node(new Node(new Node())).depth());
function first(arr) {
    var ref;
    return arr === null || arr === void 0 ? void 0 : (ref = arr[0]) === null || ref === void 0 ? void 0 : ref.value;
}
console.log(first([
    {
        value: 5
    }
]), first(undefined), first([]));
