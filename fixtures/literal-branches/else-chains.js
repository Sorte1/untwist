function chain() {
  const seen = [];
  if (0) seen.push("dead1"); else if (1) seen.push("second"); else seen.push("dead2");
  if (false) seen.push("dead3"); else if (0) seen.push("dead4"); else if (null) seen.push("dead5"); else seen.push("last");
  if (true) seen.push("first"); else if (1) seen.push("dead6");
  if (0) { seen.push("dead7"); } else if (seen.length) { seen.push("dynamic"); } else { seen.push("never reached"); }
  return seen;
}
console.log(chain().join(","));
