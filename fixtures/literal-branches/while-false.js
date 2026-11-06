function loops() {
  const seen = [];
  while (false) { seen.push("dead1"); }
  while (0) seen.push("dead2");
  while ("") { seen.push("dead3"); while (true) {} }
  while (null) { while (true) {} }
  seen.push("done");
  return seen;
}
console.log(loops().join(","));
if (false) { while (true) {} }
console.log("finished");
