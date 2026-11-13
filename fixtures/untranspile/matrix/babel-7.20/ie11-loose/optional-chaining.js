"use strict";

var _data$user, _data$empty, _data$missing, _data$user2, _data$user2$tags, _data$empty2, _get, _get$user, _data$user3, _data$empty$x, _data$empty3, _data$user4;
var data = {
  user: {
    name: "ann",
    tags: ["a"],
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
console.log((_data$user = data.user) == null ? void 0 : _data$user.name, (_data$empty = data.empty) == null ? void 0 : _data$empty.name, (_data$missing = data.missing) == null ? void 0 : _data$missing.deep.deeper);
console.log((_data$user2 = data.user) == null ? void 0 : (_data$user2$tags = _data$user2.tags) == null ? void 0 : _data$user2$tags[0], (_data$empty2 = data.empty) == null ? void 0 : _data$empty2[0], data.user.greet == null ? void 0 : data.user.greet(), data.user.nope == null ? void 0 : data.user.nope());
console.log((_get = get()) == null ? void 0 : (_get$user = _get.user) == null ? void 0 : _get$user.name, calls);
var fn = null;
console.log(fn == null ? void 0 : fn(1), typeof (fn == null ? void 0 : fn.call));
var key = "name";
console.log(data == null ? void 0 : (_data$user3 = data.user) == null ? void 0 : _data$user3[key], (_data$empty$x = (_data$empty3 = data.empty) == null ? void 0 : _data$empty3.x) != null ? _data$empty$x : "default");
console.log(data == null ? true : (_data$user4 = data.user) == null ? true : delete _data$user4.tags, data.user.tags);
var Node = /*#__PURE__*/function () {
  function Node(next) {
    this.next = next;
  }
  var _proto = Node.prototype;
  _proto.depth = function depth() {
    var _this$next$depth, _this$next;
    return 1 + ((_this$next$depth = (_this$next = this.next) == null ? void 0 : _this$next.depth()) != null ? _this$next$depth : 0);
  };
  return Node;
}();
console.log(new Node(new Node(new Node())).depth());
function first(arr) {
  var _arr$;
  return arr == null ? void 0 : (_arr$ = arr[0]) == null ? void 0 : _arr$.value;
}
console.log(first([{
  value: 5
}]), first(undefined), first([]));
