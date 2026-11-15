function f(a) { return a + 1; }
Function("a = [1", "b = eval]", "return b")()("f = function () { return 99; }");
console.log(f(1), f(2));
