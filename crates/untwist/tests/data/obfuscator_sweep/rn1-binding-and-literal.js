function g(){ let x = 0; if (true) { let x = 5; const o = {x}; let {y = x} = o; console.log(o.x, y, JSON.stringify({x, y})) } return x }
console.log(g());
