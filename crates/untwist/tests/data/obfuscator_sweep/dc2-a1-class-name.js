function g() {
  let x = 0;
  if (true) { class x {} console.log(x.name); }
  return x;
}
console.log(g());
