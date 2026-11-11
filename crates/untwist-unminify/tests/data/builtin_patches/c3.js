function gg() { return this; } gg().JSON.parse = function (s) { return s.length; };
var x = JSON.parse("[1,2]"); console.log(typeof x === "object" ? JSON.stringify(x) : x);
var y = [0].filter((v) => !!v); console.log(y);
