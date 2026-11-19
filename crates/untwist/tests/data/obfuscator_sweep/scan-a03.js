var PATCH = function () { return "'+'patched"; };
var p = ([].__proto__ ||= 0); p.toString = PATCH;
function f() { eval("console.log('or" + [] + "ig')"); } try { f(); } catch (e) { console.log(e.name); }
