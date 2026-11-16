var o = { m() { return eval("super.toString") === Object.prototype.toString; } }; console.log(o.m());
