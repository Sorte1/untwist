function lastIndex(a) { return a + 1; }
function exec() { return null; } var flags = "g";
RegExp.prototype[Symbol.replace].call(globalThis, "x", "y");
try { console.log(lastIndex(1), lastIndex(2)); } catch (e) { console.log("T"); }
