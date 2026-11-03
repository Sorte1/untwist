const words = ["lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit"];
const sentence = words.join(" ");
const title = sentence.replace(/\b\w/g, (letter) => letter.toUpperCase());
const accents = "café naïve façade";
const symbols = "tab\there, newline\\n, quote \" and 'single'";
const glyphs = "雪 → ✓";
const keepVerbatim = "keep this string literal";

function caesar(text, shift) {
  return text
    .split("")
    .map((ch) => {
      const code = ch.charCodeAt(0);
      if (code < 97 || code > 122) {
        return ch;
      }
      return String.fromCharCode(((code - 97 + shift) % 26) + 97);
    })
    .join("");
}

const template = `${words.length} words, first ${words[0]}, last ${words[words.length - 1]}`;
console.log(title);
console.log(caesar("attack at dawn", 13));
console.log(accents.toUpperCase(), accents.length);
console.log(symbols);
console.log(glyphs, glyphs.length);
console.log(template);
console.log(keepVerbatim.startsWith("keep"), "padded".padStart(10, "."), "trim me  ".trim() + "|");
console.log(["x", "y", "z"].map((s, i) => s.repeat(i + 1)).join("/"));
