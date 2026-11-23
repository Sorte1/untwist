function g(){ let x = 0; if (true) { let x = 5; ({x = 7} = {x: 1}); console.log(x) } return x }
console.log(g());
