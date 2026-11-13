class Resource {
  constructor(name) {
    this.name = name;
  }
}
class Pooled extends Resource {
  constructor(name, size = 4) {
    super(name);
    this.size = size;
    if (size > 8) {
      this.capped = true;
      return;
    }
    this.items = [];
  }
}
const [first, second] = [new Pooled("a"), new Pooled("b", 16)];
console.log(first.items.length, second.capped, second.items, first instanceof Resource);
