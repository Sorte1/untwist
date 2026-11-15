function f(a) { return a + 1; }
Function("a = '", "', b = eval", "return b")()("f = function () { return 99; }");
console.log(f(1), f(2));
