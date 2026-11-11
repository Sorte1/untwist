var { parse: orig } = JSON; (Math.random() < 2 ? JSON : 0).parse = function (s) { return s.length; };
var x = JSON.parse("[1,2]"); console.log(typeof x === "object" ? JSON.stringify(x) : x);
var y = [0].filter((v) => !!v); console.log(y);
var z = [0].find((v) => !!v); console.log(z);
