function run() {
  const seen = [];
  if (false) seen.push("dead1");
  if (0) { seen.push("dead2"); }
  if (true) seen.push("kept");
  if (1) { seen.push("kept block"); }
  seen.push("end");
  return seen;
}
console.log(run().join(","));
