"use strict";

var _data$user, _data$empty, _data$missing, _data$user2, _data$user2$tags, _data$empty2, _data$user$greet, _data$user3, _data$user$nope, _data$user4, _get, _get$user, _data$user5, _data$empty$x, _data$empty3, _data$user6;

function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function _defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function _createClass(Constructor, protoProps, staticProps) { if (protoProps) _defineProperties(Constructor.prototype, protoProps); if (staticProps) _defineProperties(Constructor, staticProps); return Constructor; }

function _typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { _typeof = function _typeof(obj) { return typeof obj; }; } else { _typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return _typeof(obj); }

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

console.log((_data$user = data.user) === null || _data$user === void 0 ? void 0 : _data$user.name, (_data$empty = data.empty) === null || _data$empty === void 0 ? void 0 : _data$empty.name, (_data$missing = data.missing) === null || _data$missing === void 0 ? void 0 : _data$missing.deep.deeper);
console.log((_data$user2 = data.user) === null || _data$user2 === void 0 ? void 0 : (_data$user2$tags = _data$user2.tags) === null || _data$user2$tags === void 0 ? void 0 : _data$user2$tags[0], (_data$empty2 = data.empty) === null || _data$empty2 === void 0 ? void 0 : _data$empty2[0], (_data$user$greet = (_data$user3 = data.user).greet) === null || _data$user$greet === void 0 ? void 0 : _data$user$greet.call(_data$user3), (_data$user$nope = (_data$user4 = data.user).nope) === null || _data$user$nope === void 0 ? void 0 : _data$user$nope.call(_data$user4));
console.log((_get = get()) === null || _get === void 0 ? void 0 : (_get$user = _get.user) === null || _get$user === void 0 ? void 0 : _get$user.name, calls);
var fn = null;
console.log(fn === null || fn === void 0 ? void 0 : fn(1), _typeof(fn === null || fn === void 0 ? void 0 : fn.call));
var key = "name";
console.log(data === null || data === void 0 ? void 0 : (_data$user5 = data.user) === null || _data$user5 === void 0 ? void 0 : _data$user5[key], (_data$empty$x = (_data$empty3 = data.empty) === null || _data$empty3 === void 0 ? void 0 : _data$empty3.x) !== null && _data$empty$x !== void 0 ? _data$empty$x : "default");
console.log(data === null || data === void 0 ? true : (_data$user6 = data.user) === null || _data$user6 === void 0 ? true : delete _data$user6.tags, data.user.tags);

var Node = /*#__PURE__*/function () {
  function Node(next) {
    _classCallCheck(this, Node);

    this.next = next;
  }

  _createClass(Node, [{
    key: "depth",
    value: function depth() {
      var _this$next$depth, _this$next;

      return 1 + ((_this$next$depth = (_this$next = this.next) === null || _this$next === void 0 ? void 0 : _this$next.depth()) !== null && _this$next$depth !== void 0 ? _this$next$depth : 0);
    }
  }]);

  return Node;
}();

console.log(new Node(new Node(new Node())).depth());

function first(arr) {
  var _arr$;

  return arr === null || arr === void 0 ? void 0 : (_arr$ = arr[0]) === null || _arr$ === void 0 ? void 0 : _arr$.value;
}

console.log(first([{
  value: 5
}]), first(undefined), first([]));
