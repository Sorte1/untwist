function outer(arr) { function h() { return 1; } { function h() { return 2; } var al = arr(); for (;;) { try { if (al[0] === "c") break; al.push(al.shift()); } catch (e) { al.push(al.shift()); } } } return h(); }
console.log(outer(function () { return ["a", "b", "c"]; }));
