var tbl = ["x", "y"]; function look(i) { return tbl[i]; }
var r = Function("return (function() {}.constructor(\"return eval\")( ));")();
console.log(look(0) + look(1), r("typeof tbl + typeof look"));
