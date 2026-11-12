var ownKeys = require("core-js/internals/own-keys");
var getBuiltIn = require("core-js/internals/get-built-in");
var keys = ownKeys({ a: 1, b: 2 });
console.log("own-keys " + keys.join(","));
console.log("built-in " + typeof getBuiltIn("Reflect", "ownKeys") + " " + typeof getBuiltIn("Array"));
