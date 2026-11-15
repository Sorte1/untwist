console.log("a");
eval("var q = 1; if (0) { [(q = 1)] = []; } console.log('in');");
console.log("b");
