function g(o){ let x = 0, z = 1; if (o) { let x = 5, z = 6; ({x, z = 8, ...o} = {x: 9, w: 2}); for ([{x}] of [[{x: 4}]]) console.log(x, z, JSON.stringify(o)); } return [x, z].join() }
console.log(g({}));
