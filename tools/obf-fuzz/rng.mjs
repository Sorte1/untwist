import { createHash } from "node:crypto";

export function seedWords(text) {
  const digest = createHash("sha256").update(String(text)).digest();
  return [0, 4, 8, 12].map((offset) => digest.readUInt32LE(offset));
}

export class Rng {
  constructor(text) {
    [this.a, this.b, this.c, this.d] = seedWords(text);
  }

  next() {
    this.a >>>= 0;
    this.b >>>= 0;
    this.c >>>= 0;
    this.d >>>= 0;
    const t = (this.a + this.b) | 0;
    this.a = this.b ^ (this.b >>> 9);
    this.b = (this.c + (this.c << 3)) | 0;
    this.c = (this.c << 21) | (this.c >>> 11);
    this.d = (this.d + 1) | 0;
    const out = (t + this.d) | 0;
    this.c = (this.c + out) | 0;
    return (out >>> 0) / 4294967296;
  }

  int(max) {
    return Math.floor(this.next() * max);
  }

  range(min, max) {
    return min + this.int(max - min + 1);
  }

  chance(p) {
    return this.next() < p;
  }

  pick(items) {
    return items[this.int(items.length)];
  }

  shuffle(items) {
    const out = [...items];
    for (let i = out.length - 1; i > 0; i--) {
      const j = this.int(i + 1);
      [out[i], out[j]] = [out[j], out[i]];
    }
    return out;
  }

  sample(items, count) {
    return this.shuffle(items).slice(0, count);
  }

  word() {
    const letters = "abcdefghijklmnopqrstuvwxyz";
    let out = "";
    const length = this.range(3, 8);
    for (let i = 0; i < length; i++) out += letters[this.int(letters.length)];
    return out;
  }
}
