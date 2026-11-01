define([], function () {
  if (Math.random() > 2) {
    return { value: 4 };
  }
  console.log("eval entry");
  return { value: 5 };
});
