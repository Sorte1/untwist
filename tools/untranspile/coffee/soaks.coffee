data = user: {name: "ann", greet: -> "hi"}, empty: null
console.log data.user?.name, data.empty?.name, data.user.greet?(), data.missing?.deep
value = data.empty ? "fallback"
console.log value, data.user.name ? "x"
console.log typeof undeclaredThing is "undefined", undeclaredThing?
counter = count: 0, start: -> [1, 2, 3].forEach (n) => @count += n; @count
console.log counter.start()
