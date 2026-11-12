sum = (first, rest...) -> first + rest.reduce(((a, b) -> a + b), 0)
nums = [1, 2, 3]
console.log sum(nums...), sum(0, nums...), [nums..., 4].join(",")
[a, b, rest...] = [1, 2, 3, 4]
{x, y: why = 5} = {x: 1}
console.log a, b, rest.join(","), x, why
squares = (n * n for n in nums when n isnt 2)
console.log squares.join(","), (k for own k of {p: 1, q: 2}).join(",")
console.log 2 ** 10, "#{a} and #{b}", 3 in nums
