define([], function () {
  console.log("eval entry", typeof this);
  return { value: 1 };
});
