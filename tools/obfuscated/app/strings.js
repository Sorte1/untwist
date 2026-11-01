export function shout(text) {
  return text.split(" ").map((word) => word.toUpperCase()).join("! ") + "!";
}

export default class Greeter {
  constructor(name) {
    this.name = name;
    this.greeting = "welcome back";
  }

  greet() {
    return `${this.greeting}, ${this.name} (${this.name.length} chars)`;
  }
}
