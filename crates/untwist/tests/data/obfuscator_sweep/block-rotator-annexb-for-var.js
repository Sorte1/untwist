function outer(arr) { for (var h = 0; h < 1; h++) { } { function h() { return 2; } var al = arr(); for (;;) { try { if (al[0] === "c") break; al.push(al.shift()); } catch (e) { al.push(al.shift()); } } } return typeof h; }
console.log(outer(function () { return ["a", "b", "c"]; }));
