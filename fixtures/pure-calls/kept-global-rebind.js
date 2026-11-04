function dec(a) {
  return ["a", "b"][a];
}
globalThis[Object.keys({ dec: 1 })[0]] = function () {
  return "z";
};
console.log(dec(0));
