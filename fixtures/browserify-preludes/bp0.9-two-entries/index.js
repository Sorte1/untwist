(function(e,t,n){function i(n,s){if(!t[n]){if(!e[n]){var o=typeof require=="function"&&require;if(!s&&o)return o(n,!0);if(r)return r(n,!0);throw new Error("Cannot find module '"+n+"'")}var u=t[n]={exports:{}};e[n][0].call(u.exports,function(t){var r=e[n][1][t];return i(r?r:t)},u,u.exports)}return t[n].exports}var r=typeof require=="function"&&require;for(var s=0;s<n.length;s++)i(n[s]);return i})({1:[function(require,module,exports){
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

},{"./a":2}],4:[function(require,module,exports){
var a = require("./lib/a");
console.log("second entry", a.count);

},{"./lib/a":2}]},{},[1,4])