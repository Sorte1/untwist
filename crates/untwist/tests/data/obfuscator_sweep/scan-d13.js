var Fn; (Fn = Function, Object).getPrototypeOf(0); Fn("atob = function () { return \"patched\"; }")();
console.log(atob("aGk="));
