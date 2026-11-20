function outer(arr) { var h = 1; { var al = arr(); function h() { return 2; } while (true) { try { var n = parseInt(al[0]); if (n === 3) break; al.push(al.shift()); } catch (e) { al.push(al.shift()); } } } return typeof h; }
console.log(outer(function () { return ["1", "2", "3"]; }));
