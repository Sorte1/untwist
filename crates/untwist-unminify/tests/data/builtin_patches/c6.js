(function () { this.JSON.parse = function (s) { return s.length; }; }).call(void 0);
var x = JSON.parse("[1,2]"); console.log(typeof x === "object" ? JSON.stringify(x) : x);
var y = [0].filter((v) => !!v); console.log(y);
