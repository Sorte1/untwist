function f(a) { return a + 1; }
eval("if (0) g() = 1; globalThis['Ref' + 'lect'].set(globalThis, 'f', function () { return 99; })");
console.log(f(1), f(2));
