!function(e,r,t,o,n){var a="u">typeof globalThis?globalThis:"u">typeof self?self:"u">typeof window?window:"u">typeof global?global:{},s="function"==typeof a[o]&&a[o],l=s.i||{},u=s.cache||{},c="u">typeof module&&"function"==typeof module.require&&module.require.bind(module);function f(r,t){if(!u[r]){if(!e[r]){if(n[r])return n[r];var l="function"==typeof a[o]&&a[o];if(!t&&l)return l(r,!0);if(s)return s(r,!0);if(c&&"string"==typeof r)return c(r);var i=Error("Cannot find module '"+r+"'");throw i.code="MODULE_NOT_FOUND",i}d.resolve=function(t){var o=e[r][1][t];return null!=o?o:t},d.cache={};var p=u[r]=new f.Module(r);e[r][0].call(p.exports,d,p,p.exports,a)}return u[r].exports;function d(e){var r=d.resolve(e);if(!1===r)return{};if(Array.isArray(r)){var t={__esModule:!0};return r.forEach(function(e){var r=e[0],o=e[1],n=e[2]||e[0],a=f(o);"*"===r?Object.keys(a).forEach(function(e){"default"===e||"__esModule"===e||Object.prototype.hasOwnProperty.call(t,e)||Object.defineProperty(t,e,{enumerable:!0,get:function(){return a[e]}})}):"*"===n?Object.defineProperty(t,r,{enumerable:!0,value:a}):Object.defineProperty(t,r,{enumerable:!0,get:function(){return"default"===n?a.__esModule?a.default:a:a[n]}})}),t}return f(r)}}f.isParcelRequire=!0,f.Module=function(e){this.id=e,this.bundle=f,this.require=c,this.exports={}},f.modules=e,f.cache=u,f.parent=s,f.distDir=void 0,f.publicUrl=void 0,f.devServer=void 0,f.i=l,f.register=function(r,t){e[r]=[function(e,r){r.exports=t},{}]},Object.defineProperty(f,"root",{get:function(){return a[o]}}),a[o]=f;for(var i=0;i<r.length;i++)f(r[i]);if(t){var p=f(t);"object"==typeof exports&&"u">typeof module?module.exports=p:"function"==typeof define&&define.amd&&define(function(){return p})}}({a:[function(require,module,exports,__globalThis){
var b = require("./b.js"), c = require("./c.js"), path = require("4f1c2a9b7d3e5f60");
console.log("eval a");
console.log("b " + b.name + " " + b.twice(4) + " " + b.self);
console.log("c " + c.data.items.length + " " + c.tag);
console.log("cycle " + require("./x.js").report());
console.log("path " + path.posix.join("x", "y"));
console.log("global " + (__globalThis === globalThis) + " " + typeof module + " " + (this === exports));
Promise.resolve(module.bundle.root("e")).then(function (m) { console.log("lazy " + m.value); });
},{"./b.js":"b","./c.js":"c","./x.js":"x","4f1c2a9b7d3e5f60":"path"}],
b:[function(e,r,t,o){console.log("eval b");t.name="bee";t.twice=function(n){return 2*n};t.self=o===globalThis},{}],
c:[function(e,r,t){console.log("eval c");var j=e("./j.json"),b=e("./b.js");r.exports={data:j,tag:"c-"+b.name+"-"+typeof r}},{"./j.json":"j","./b.js":"b"}],
j:[function(e,r){r.exports=JSON.parse('{"items":[1,2,3]}')},{}],
x:[function(e,r,t){console.log("eval x start");t.early="x-early";var y=e("./y.js");t.late="x-late";t.report=function(){return y.seen+" "+y.check()};console.log("eval x end")},{"./y.js":"y"}],
y:[function(e,r,t){console.log("eval y");var x=e("./x.js");t.seen="y saw "+x.early+","+x.late;t.check=function(){return "y later "+x.late}},{"./x.js":"x"}],
e:[function(require,module,exports){console.log("eval e");exports.value=42},{}]},["a"],"a","parcelRequire94c2",{});
