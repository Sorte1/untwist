var PATCH = function () { return "'+'patched"; };
var C = ([].constructor ||= 0); C.prototype.toString = PATCH;
function f() { eval("console.log('or" + [] + "ig')"); } try { f(); } catch (e) { console.log(e.name); }
