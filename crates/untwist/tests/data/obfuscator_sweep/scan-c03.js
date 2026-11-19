var PATCH = function () { return "'+'patched"; };
typeof 0[[].__proto__.toString = PATCH];
function f() { eval("console.log('or" + [] + "ig')"); } try { f(); } catch (e) { console.log(e.name); }
