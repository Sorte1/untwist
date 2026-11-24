var __assign = (this && this.__assign) || function () { __assign = Object.assign || function(t) { for (var s, i = 1, n = arguments.length; i < n; i++) { s = arguments[i]; for (var p in s) if (Object.prototype.hasOwnProperty.call(s, p)) t[p] = s[p]; } return t; }; return __assign.apply(this, arguments); };
function show(o){return Object.keys(o).join()+'|'+(Object.getPrototypeOf(o)===Object.prototype)+'|'+o.zz}
var other={zz:1};
(function(){ var __proto__ = other;
console.log(show(__assign({}, {__proto__})));
console.log(show(__assign({}, {["__proto__"]: other})));
console.log(show(__assign({}, {"__proto__": other})));
console.log(show(__assign({a: 1}, {b: 2, ["__proto__"]: other})));
console.log(show(__assign({}, {__proto__: other})));
})();
