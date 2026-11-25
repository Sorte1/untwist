const corpus = [{ anchor: "first", payload: "one\ntwo" }, { anchor: "second", payload: "drei" }];
function digest(seedInput = 0) {
  if (!Array.isArray(corpus) || corpus.length < 2) return 0;
  let acc = ((seedInput | 0) ^ 0x26b5c67b) >>> 0;
  for (let i = 0; i < corpus.length; i++) {
    const entry = corpus[i];
    if (!entry || typeof entry.anchor !== "string" || typeof entry.payload !== "string") return 0;
    const text = entry.anchor + "::" + entry.payload;
    for (let j = 0; j < text.length; j++) {
      acc = Math.imul(acc ^ text.charCodeAt(j), 0x85ebca6b) >>> 0;
      acc = (acc << 13 | acc >>> 19) >>> 0;
    }
  }
  return acc >>> 0;
}
const seed = Date.now() & 0x7fffc | (digest(0) === 12345 ? 1 : 0);
console.log(((Math.imul(seed ^ 8, seed ^ 8) & 7) === 1 ? "a" : "b") + ((Math.imul(seed ^ 3, seed ^ 3) & 7) === 0 ? "c" : "d"));
