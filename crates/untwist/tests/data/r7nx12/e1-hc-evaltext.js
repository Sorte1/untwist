function hole(v) { var k = ['constructor'][0]; return v[k]; }
hole([]).prototype.join = function () { return 'X'; };
function f() { eval("console.log('or" + [] + "ig')"); } f();
