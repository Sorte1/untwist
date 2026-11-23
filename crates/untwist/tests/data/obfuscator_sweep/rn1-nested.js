function g(){ let x = 0; if (true) { let x = 5; [{x}] = [{x: 3}]; console.log(x); ({a: {x}} = {a: {x: 4}}); console.log(x); } return x }
console.log(g());
