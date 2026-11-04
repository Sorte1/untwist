let early;
try {
  early = show();
} catch (e) {
  early = e.constructor.name;
}
console.log(early);
const dec = (a) => ["a", "b"][a];
function show() {
  return dec(1);
}
console.log(show());
