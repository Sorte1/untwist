((0, eval)("atob = function () { return \"patched\"; }"), Object).getPrototypeOf(0);
console.log(atob("aGk="));
