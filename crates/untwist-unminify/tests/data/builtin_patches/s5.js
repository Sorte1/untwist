(function () { var set = () => { var k = "JS" + "ON"; this[k].parse = function (s) { return s.length; }; }; set(); })();
var x = JSON.parse("[1,2]"); console.log(typeof x === "object" ? JSON.stringify(x) : x);
var y = [0].filter((v) => !!v); console.log(y);
