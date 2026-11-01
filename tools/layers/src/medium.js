function area(width, height) { return width * height; }
var shapes = [{ name: 'square', width: 2, height: 2 }, { name: 'strip', width: 5, height: 1 }];
shapes.forEach(function (shape) { console.log(shape.name, area(shape.width, shape.height)); });
console.log('total', shapes.reduce(function (sum, shape) { return sum + area(shape.width, shape.height); }, 0));
