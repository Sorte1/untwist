var o9 = { a: 1 }; function names9(){ var n = []; for (var k in o9) if (k.length < 8) n.push(k); return n.sort().join() }
