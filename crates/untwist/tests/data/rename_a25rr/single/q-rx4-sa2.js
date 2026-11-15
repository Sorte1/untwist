var tbl = ["x", "y"]; function look(i) { return tbl[i]; }
console.log(look(0) + look(1), Function("return eval")()("typeof tbl + typeof look"));
