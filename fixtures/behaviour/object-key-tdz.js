function run() {
  var O = { a: 1, b: 2 };
  var read;
  try {
    read = O[key];
  } catch (error) {
    read = error.constructor.name;
  }
  const key = "a";
  return [read, O.b].join(",");
}
console.log(run());
