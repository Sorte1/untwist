class Animal {
  constructor(name, legs) {
    this.name = name;
    this.legs = legs;
  }

  describe() {
    return this.name + " walks on " + this.legs + " legs";
  }

  static compare(left, right) {
    return left.legs - right.legs;
  }

  get kind() {
    return "animal";
  }
}

class Bird extends Animal {
  constructor(name) {
    super(name, 2);
    this.wings = 2;
  }

  describe() {
    return super.describe() + " and flies with " + this.wings + " wings";
  }

  get kind() {
    return "bird";
  }
}

class Tally {
  count = 0;

  add(amount) {
    this.count += amount;
    return this;
  }
}

const zoo = [new Animal("cat", 4), new Bird("crow"), new Animal("snail", 0)];
zoo.sort(Animal.compare);
for (const creature of zoo) {
  console.log(creature.describe(), creature.kind);
}
console.log("is bird", zoo[1] instanceof Bird, zoo[1] instanceof Animal);
console.log("tally", new Tally().add(3).add(4).count);
