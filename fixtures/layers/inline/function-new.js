var greet = new Function("name", "return 'hello ' + name");
console.log("inline new Function", greet("layers"), typeof greet);
