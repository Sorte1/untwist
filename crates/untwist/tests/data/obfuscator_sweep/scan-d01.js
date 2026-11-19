var PATCH = function () { return "'+'patched"; };
delete Promise.resolve[[].__proto__.toString = PATCH];
function f() { eval("console.log('or" + [] + "ig')"); } try { f(); } catch (e) { console.log(e.name); }
