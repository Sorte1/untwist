function g(){ let x = 0; if (true) { let x = 5; ({x, ...x} = {x: 9, y: 1}); console.log(JSON.stringify(x)) } return x }
console.log(g());
