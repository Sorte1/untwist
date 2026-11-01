const fs = require('fs');
const cases = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const render = (value) => {
  if (typeof value === 'number') {
    return Object.is(value, -0) ? 'number:-0' : `number:${String(value)}`;
  }
  if (typeof value === 'string') return `string:${value}`;
  if (typeof value === 'boolean') return `boolean:${value}`;
  return String(value);
};
const out = cases.map((c) => {
  try {
    switch (c.op) {
      case 'to_number':
        return render(Number(c.a));
      case 'to_string':
        return render(String(c.a));
      case 'parse_int':
        return render(c.radix === null ? parseInt(c.a) : parseInt(c.a, c.radix));
      case 'to_int32':
        return render(c.a | 0);
      case 'add':
        return render(c.a + c.b);
      case 'loose_eq':
        return render(c.a == c.b);
      case 'decode_uri_component':
        return render(decodeURIComponent(c.a));
      default:
        return 'unknown-op';
    }
  } catch (e) {
    return `throw:${e.constructor.name}`;
  }
});
process.stdout.write(JSON.stringify(out));
