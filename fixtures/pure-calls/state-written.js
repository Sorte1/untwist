let count = 0;
const dec = (a) => (count++, ["a", "b"][a]);
console.log(dec(0), dec(1), count);
