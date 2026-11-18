var PATCH = function () { return "'+'patched"; };
var o = []; var k = "__proto__"; var p = (o[k] ||= 0); p.toString = PATCH;
function f() { eval("console.log('or" + [] + "ig')"); } try { f(); } catch (e) { console.log(e.name); }
