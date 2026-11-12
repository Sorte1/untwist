class Animal
  constructor: (@name) ->
  speak: -> "#{@name} makes a sound"
  @create: (name) -> new this(name)

class Dog extends Animal
  constructor: (name, @breed = "mutt") ->
    super(name)
  speak: -> super.speak() + " (woof)"

d = new Dog("Rex")
console.log d.speak(), d.breed, d instanceof Animal, Dog.create("Max").speak()
