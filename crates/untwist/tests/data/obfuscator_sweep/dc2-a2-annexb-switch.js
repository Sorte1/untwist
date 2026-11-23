function g() { var x = 1; switch (1) { case 1: if (true) { let x = 4; { function x() {} } console.log(x); } } return typeof x; }
console.log(g());
