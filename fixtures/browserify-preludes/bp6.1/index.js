(function(){function r(e,n,t){function o(i,f){if(!n[i]){if(!e[i]){var c="function"==typeof require&&require;if(!f&&c)return c(i,!0);if(u)return u(i,!0);var a=new Error("Cannot find module '"+i+"'");throw a.code="MODULE_NOT_FOUND",a}var p=n[i]={exports:{}};e[i][0].call(p.exports,function(r){var n=e[i][1][r];return o(n||r)},p,p.exports,r,e,n,t)}return n[i].exports}for(var u="function"==typeof require&&require,i=0;i<t.length;i++)o(t[i]);return o}return r})()({1:[function(require,module,exports){
var a = require("./lib/a");
var b = require("./lib/b.js");
console.log("entry", a.name, b.describe(), this === module.exports);
module.exports = { total: a.count + b.count };
console.log("entry exports", JSON.stringify(module.exports));

},{"./lib/a":2,"./lib/b.js":3}],2:[function(require,module,exports){
exports.name = "a";
exports.count = 1;
var b = require("./b");
console.log("a sees b", typeof b.describe);

},{"./b":3}],3:[function(require,module,exports){
var a = require("./a");
console.log("b sees a", a.name);
module.exports = {
  count: 2,
  describe: function () {
    return "b with " + a.name;
  },
};

},{"./a":2}]},{},[1]);
