function greet(name) {
  return "hello, " + name;
}

var guests = ["ada", "grace", "linus"];
for (var i = 0; i < guests.length; i++) {
  console.log(greet(guests[i]));
}
console.log("done with " + guests.length + " guests");
