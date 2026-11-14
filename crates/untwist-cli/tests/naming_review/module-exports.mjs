export var f = (console.log("h"), function () {});
export var g = true ? function () {} : 0;
export var o = {}; o.p = function () {};
var d = 0 || function () {};
export { d as default };
