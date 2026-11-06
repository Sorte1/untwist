function pick() {
  const seen = [];
  if (true) seen.push("true"); else seen.push("dead1");
  if (1) seen.push("one"); else seen.push("dead2");
  if (-1) seen.push("negative one"); else seen.push("dead3");
  if ("a") seen.push("string"); else seen.push("dead4");
  if (!0) seen.push("not zero"); else seen.push("dead5");
  if (!!1) seen.push("not not one"); else seen.push("dead6");
  if (!null) seen.push("not null"); else seen.push("dead7");
  if (" ") seen.push("space"); else seen.push("dead8");
  return seen;
}
console.log(pick().join(","));
