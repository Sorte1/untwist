function f(a) { return a + 1; }
Function("a = [eval", "0][0]", "return a")()("f = function () { return 99; }");
console.log(f(1), f(2));
