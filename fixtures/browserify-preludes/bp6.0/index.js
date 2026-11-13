(function(){function e(t,n,r){function s(o,u){if(!n[o]){if(!t[o]){var a=typeof require=="function"&&require;if(!u&&a)return a(o,!0);if(i)return i(o,!0);var f=new Error("Cannot find module '"+o+"'");throw f.code="MODULE_NOT_FOUND",f}var l=n[o]={exports:{}};t[o][0].call(l.exports,function(e){var n=t[o][1][e];return s(n?n:e)},l,l.exports,e,t,n,r)}return n[o].exports}var i=typeof require=="function"&&require;for(var o=0;o<r.length;o++)s(r[o]);return s}return e})()({1:[function(require,module,exports){
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
