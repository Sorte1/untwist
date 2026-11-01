define(["require"], function (localRequire) {
  console.log("eval entry", typeof localRequire);
  return { value: 3 };
});
