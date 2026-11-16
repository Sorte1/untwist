import itertools
D = ["", "let x;", "const x = 0;", "class x {}", "function x(){}", "var x;", "async function x(){}", "function* x(){}"]
S = [
  ("blk", "{ %s }"),
  ("catch", "try {} catch (x) { %s }"),
  ("catchp", "try {} catch ([x]) { %s }"),
  ("catchy", "try {} catch (y) { %s }"),
  ("forlet", "for (let x;;) { %s break; }"),
  ("forin", "for (let x in {}) { %s }"),
  ("forvar", "for (var y;;) { %s break; }"),
  ("switch", "switch (0) { case 0: %s }"),
  ("fn", "function f(x) { %s }"),
  ("arrow", "(() => { %s });"),
  ("static", "class C { static { %s } }"),
  ("label", "l: { %s }"),
  ("if", "if (1) { %s }"),
]
I = ["var x;", "for (var x in {});", "var [x] = [];", "function x(){}", "let x;", "try {} catch (x) { var x; }"]
rows = set()
for (n1, s1), (n2, s2), inner in itertools.product(S, S, I):
    for pos in range(4):
        for d in (D if pos < 3 else [""]):
            if pos < 3 and d == "":
                continue
            top = d if pos == 0 else ""
            d1 = d if pos == 1 else ""
            d2 = d if pos == 2 else ""
            src = (top + " " + (s1 % (d1 + " " + (s2 % (d2 + " " + inner))))).strip()
            rows.add(src)
for (n1, s1), inner in itertools.product(S, I):
    for d in D:
        rows.add((d + " " + (s1 % inner)).strip())
        rows.add((s1 % (d + " " + inner)).strip())
        rows.add((s1 % (inner + " " + d)).strip())
        rows.add(((s1 % inner) + " " + d).strip())
for r in sorted(rows):
    print(r)
