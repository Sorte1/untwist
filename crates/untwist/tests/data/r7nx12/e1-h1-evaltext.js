function hole(v) { var arguments; arguments.toString = function () { return 'constructor'; }; return v[arguments]; }
hole([]).prototype.join = function () { return 'X'; };
function f() { eval("console.log('or" + [] + "ig')"); } f();
