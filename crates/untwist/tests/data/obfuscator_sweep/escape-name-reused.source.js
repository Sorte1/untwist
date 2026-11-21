class Pet {
  constructor(name, legs) {
    this.name = name;
    this.legs = legs;
  }
  static compare(a, b) {
    return a.legs - b.legs;
  }
}
const pets = [new Pet("cat", 4), new Pet("crow", 2)];
pets.sort(Pet.compare);
console.log(pets[0].name, pets[1].legs);
