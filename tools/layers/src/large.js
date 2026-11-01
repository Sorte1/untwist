var inventory = { apples: 3, bananas: 7, cherries: 11, dates: 13, elderberries: 17, figs: 19 };
function describe(name, count) { return name + ' x' + count; }
var keys = Object.keys(inventory);
var lines = keys.map(function (key) { return describe(key, inventory[key]); });
var stock = keys.reduce(function (total, key) { return total + inventory[key]; }, 0);
var report = { first: keys[0], last: keys[keys.length - 1], longest: keys.slice().sort(function (left, right) { return right.length - left.length; })[0] };
console.log(lines.join('; '));
console.log('stock', stock, 'first', report.first, 'last', report.last, 'longest', report.longest);
var matrix = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];
var diagonal = matrix.map(function (row, index) { return row[index]; });
var transposed = matrix[0].map(function (unused, column) { return matrix.map(function (row) { return row[column]; }); });
console.log('diagonal', diagonal.join(','), 'transposed', JSON.stringify(transposed));
var words = 'quick brown fox jumps over lazy dog'.split(' ');
console.log('words', words.length, words.filter(function (word) { return word.length > 3; }).join('-'));
