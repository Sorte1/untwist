"use strict";
class Shape {
    area() {
        return 0;
    }
    static of(kind) {
        return new Shape(kind);
    }
    constructor(kind){
        this.kind = kind;
    }
}
class Square extends Shape {
    area() {
        return this.side * this.side;
    }
    constructor(side){
        super("square");
        this.side = side;
    }
}
const s = new Square(3);
const seen = [];
for(const key in s)seen.push(key);
console.log(seen.join(","), Object.keys(Shape.prototype).length, Object.getOwnPropertyNames(Square.prototype).join(","));
console.log(Object.getOwnPropertyDescriptor(Shape, "prototype").writable, Object.getOwnPropertyDescriptor(Shape.prototype, "area").enumerable);
for (const make of [
    ()=>Shape("x"),
    ()=>Square(1)
]){
    try {
        make();
        console.log("called without new");
    } catch (e) {
        console.log(e.constructor.name);
    }
}
console.log(Shape.length, Square.length, typeof Shape.prototype.area.prototype);
