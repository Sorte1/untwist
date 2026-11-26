let count = 2;
const label = "items";
class Box {
  constructor(size) {
    this.size = size;
  }
}
console.log(count, label, new Box(3).size);
