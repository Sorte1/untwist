import json, sys, re
ucd = sys.argv[1]
props = {}
for line in open(f"{ucd}/PropertyAliases.txt"):
    line = line.split('#')[0].strip()
    if not line: continue
    names = [x.strip() for x in line.split(';')]
    props[names[1]] = names
values = {}
for line in open(f"{ucd}/PropertyValueAliases.txt"):
    line = line.split('#')[0].strip()
    if not line: continue
    parts = [x.strip() for x in line.split(';')]
    key = parts[0]
    vals = parts[1:]
    if key == 'ccc':
        vals = parts[2:]
    values.setdefault(key, []).append(vals)
extra = json.load(open(sys.argv[2]))
rows = []
seen = set()
def add(p, flags):
    for f in flags:
        if (p, f) not in seen:
            seen.add((p, f)); rows.append((p, f))
alnum = [chr(c) for c in range(48,58)] + [chr(c) for c in range(65,91)] + [chr(c) for c in range(97,123)]
for c in alnum:
    add('\\' + c, ['', 'u', 'v'])
    add('[\\' + c + ']', ['', 'u', 'v'])
    add('a\\' + c + '{1}', ['', 'u', 'v'])
for p in ['\\cA','\\cz','\\c1','\\c_','[\\c1]','[\\c_]','[\\cA]','\\x4','\\x41','\\u004','\\u0041','\\u{41}','\\u{110000}','\\u{10FFFF}','\\k','\\k<a>','(?<a>.)\\k<a>','\\1','(a)\\1','\\8','\\00','\\08','[\\b]','[\\B]','\\B+','\\b+','\\A+','\\z*','[\\A-\\z]','\\p','\\p{','\\p{}','\\P{L','\\q{a}','[\\q{a}]','[\\q{ab|c}]']:
    add(p, ['', 'u', 'v'])
nums = ['0','1','2147483646','2147483647','2147483648','3000000000','4294967295','4294967296','9007199254740991','9007199254740992','99999999999999999999']
for n in nums:
    add('a{%s}' % n, ['', 'u', 'v'])
    add('a{%s,}' % n, ['', 'u', 'v'])
    for m in nums:
        add('a{%s,%s}' % (n, m), ['', 'u', 'v'])
gc = sorted({v for vs in values['gc'] for v in vs})
sc = sorted({v for vs in values['sc'] for v in vs})
binary_props = [k for k, vs in values.items() if sorted(sum(vs, [])) == sorted(['N','No','F','False','Y','Yes','T','True'])]
names = set(gc) | set(sc)
for k, ns in props.items():
    names |= set(ns)
names |= set(extra)
keys = ['gc','General_Category','sc','Script','scx','Script_Extensions']
for n in sorted(names):
    for neg in 'pP':
        add('\\%s{%s}' % (neg, n), ['u', 'v'])
    add('[\\p{%s}]' % n, ['u', 'v'])
for k in keys:
    for v in sorted(set(gc) | set(sc) | set(extra)):
        add('\\p{%s=%s}' % (k, v), ['u', 'v'])
    add('\\P{%s=%s}' % (k, 'Hira'), ['u', 'v'])
for k, ns in props.items():
    vs = values.get(ns[0]) or values.get(ns[1]) or []
    for name in ns:
        for vals in vs[:2]:
            for v in vals[:2]:
                add('\\p{%s=%s}' % (name, v), ['u', 'v'])
for k in binary_props:
    for v in ['Y','N','T','F','Yes','No','True','False']:
        add('\\p{%s=%s}' % (k, v), ['u'])
for p in ['\\p{Lu}\\p{WSpace}', '\\p{WSpace}\\A', '[^\\p{RGI_Emoji}]', '\\P{RGI_Emoji}', '[\\p{RGI_Emoji}--a]', '\\p{sc=Hrkt}\\p{L}']:
    add(p, ['u', 'v'])
for p, f in rows:
    print(json.dumps({"p": p, "f": f}))
