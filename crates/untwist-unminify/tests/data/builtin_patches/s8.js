var o = { get g() { return this; } }; var k = "JS" + "ON"; Object.getOwnPropertyDescriptor(o, "g").get.call(null)[k].parse = function (s) { return s.length; };
var x = JSON.parse("[1,2]"); console.log(typeof x === "object" ? JSON.stringify(x) : x);
var y = [0].filter((v) => !!v); console.log(y);
