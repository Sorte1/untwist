(function() {
  var Animal, Dog, d;

  Animal = class Animal {
    constructor(name1) {
      this.name = name1;
    }

    speak() {
      return `${this.name} makes a sound`;
    }

    static create(name) {
      return new this(name);
    }

  };

  Dog = class Dog extends Animal {
    constructor(name, breed = "mutt") {
      super(name);
      this.breed = breed;
    }

    speak() {
      return super.speak() + " (woof)";
    }

  };

  d = new Dog("Rex");

  console.log(d.speak(), d.breed, d instanceof Animal, Dog.create("Max").speak());

}).call(this);
