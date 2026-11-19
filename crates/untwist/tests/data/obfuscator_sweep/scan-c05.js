var PATCH = function () { return "'+'patched"; };
([].__proto__.toString = PATCH, Reflect).getPrototypeOf({});
function f() { eval("console.log('or" + [] + "ig')"); } try { f(); } catch (e) { console.log(e.name); }
