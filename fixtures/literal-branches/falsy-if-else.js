function pick() {
  const seen = [];
  if (false) seen.push("dead1"); else seen.push("false");
  if (0) seen.push("dead2"); else seen.push("zero");
  if (-0) seen.push("dead3"); else seen.push("negative zero");
  if ("") seen.push("dead4"); else seen.push("empty string");
  if (null) seen.push("dead5"); else seen.push("null");
  if (void 0) seen.push("dead6"); else seen.push("void");
  if (undefined) seen.push("dead7"); else seen.push("undefined");
  if (!1) seen.push("dead8"); else seen.push("not one");
  if (!"x") seen.push("dead9"); else seen.push("not string");
  return seen;
}
console.log(pick().join(","));
