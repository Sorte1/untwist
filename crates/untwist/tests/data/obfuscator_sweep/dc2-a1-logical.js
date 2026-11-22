function g() {
  let x = 0;
  if ("a" === "a") { let x; x ||= () => {}; console.log(x.name); }
  return x;
}
console.log(g());
