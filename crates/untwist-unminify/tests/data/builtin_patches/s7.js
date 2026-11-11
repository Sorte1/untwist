function h(t) { var k = "JS" + "ON"; t[k].parse = function (s) { return s.length; }; } h(function () { return this; }());
var x = JSON.parse("[1,2]"); console.log(typeof x === "object" ? JSON.stringify(x) : x);
var y = [0].filter((v) => !!v); console.log(y);
