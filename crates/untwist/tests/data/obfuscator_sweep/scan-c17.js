var PATCH = function () { return "'+'patched"; };
var p; (p = [].__proto__, Object).getPrototypeOf(0); p.toString = PATCH;
function f() { eval("console.log('or" + [] + "ig')"); } try { f(); } catch (e) { console.log(e.name); }
