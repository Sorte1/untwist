function f() { var x = 1; var y = 2; return [x, y, setTimeout.length]; }
var t = "typeof a + typeof b";
console.log(f(), Function("return " + t)());
