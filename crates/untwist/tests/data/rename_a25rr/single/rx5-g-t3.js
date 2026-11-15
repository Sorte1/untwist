function f(a) { return a + 1; }
Function("a = (0", "eval)", "return a")()("f = function () { return 99; }");
console.log(f(1), f(2));
