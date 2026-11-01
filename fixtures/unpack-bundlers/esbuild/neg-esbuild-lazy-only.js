var y = (q, K) => () => q;
var only = y(() => 1);
console.log("only", only()());
