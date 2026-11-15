var f = "outer";
function t(){ { function f(){ return 1 } } return typeof f; }
console.log(await (async () => t())());
