class Animal {
  constructor(name, legs) {
    this.name = name;
    this.legs = legs;
  }
  describe() {
    return this.name + " walks on " + this.legs + " legs";
  }
  static compare(a, b) {
    return a.legs - b.legs;
  }
}
const pets = [new Animal("cat", 4), new Animal("crow", 2)];
pets.sort(Animal.compare);
for (const pet of pets) console.log(pet.describe());
