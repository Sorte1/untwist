function g() { return typeof f; } eval("let t = 1; console.log(g()); function f() { return t; }");
