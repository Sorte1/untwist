"use strict";
class Animal {
    speak() {
        return this.name + " makes a sound";
    }
    static create(name) {
        return new this(name);
    }
    get kind() {
        return "animal";
    }
    constructor(name){
        this.name = name;
    }
}
class Dog extends Animal {
    speak() {
        return super.speak() + " (woof)";
    }
    get kind() {
        return "dog:" + super.kind;
    }
    static create(name) {
        const d = super.create(name);
        d.breed = "mutt";
        return d;
    }
    constructor(name, breed){
        super(name);
        this.breed = breed;
    }
}
class Puppy extends Dog {
}
const d = new Dog("Rex", "lab");
console.log(d.speak(), d.kind, d instanceof Animal, d.breed);
const m = Dog.create("Max");
console.log(m instanceof Dog, m.breed, m.speak());
const pup = new Puppy("Bit", "pug");
console.log(pup.speak(), Object.getPrototypeOf(Puppy) === Dog, Puppy.name);
function Legacy(v) {
    this.v = v;
}
Legacy.prototype.get = function() {
    return this.v;
};
class Modern extends Legacy {
    get() {
        return super.get() * 2;
    }
}
console.log(new Modern(21).get());
