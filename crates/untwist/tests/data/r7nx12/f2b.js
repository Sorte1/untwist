function hole(v) { var k; { function k() {} } k.toString = function () { return 'constructor'; }; return v[k]; }
hole([]).prototype.join = function () { return 'X'; };
function f() { eval("console.log('or" + [] + "ig')"); } f();
