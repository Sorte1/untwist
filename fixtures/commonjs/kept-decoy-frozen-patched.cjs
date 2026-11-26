Object.freeze = function (value) {
  console.log("freeze ran");
  return value;
};
const decoyFrozen = Object.freeze([1, 2]);
console.log("done");
