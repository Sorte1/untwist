function g(){ let x = 0; if (true) { let x = 5; for ({x} of [{x: 3}]) console.log(x); } return x }
console.log(g());
