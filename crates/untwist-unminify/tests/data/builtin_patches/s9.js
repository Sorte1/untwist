(function () { var k = "JS" + "ON"; this.globalThis[k].parse = function (s) { return s.length; }; })();
var x = JSON.parse("[1,2]"); console.log(typeof x === "object" ? JSON.stringify(x) : x);
