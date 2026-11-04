const t = ["a", "b"];
const dec = (a) => t[a];
function mutate(list) {
  list[0] = "z";
}
mutate(t);
console.log(dec(0));
