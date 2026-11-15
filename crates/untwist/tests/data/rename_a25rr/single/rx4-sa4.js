var tbl = ["x", "y"]; function look(i) { return tbl[i]; }
var g = Function("return (function() {}.constructor(\"return this\")( ));")();
console.log(look(0) + look(1), g.eval("typeof tbl + typeof look"));
