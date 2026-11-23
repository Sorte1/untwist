function g(){ let x = 0; if (true) { let x = 5; ({x} = {x: 9}); console.log(x) } return x }
console.log(g());
