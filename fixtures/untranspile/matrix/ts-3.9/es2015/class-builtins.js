"use strict";
class AppError extends Error {
    constructor(message, code) {
        super(message);
        this.name = "AppError";
        this.code = code;
    }
    describe() {
        return this.name + "[" + this.code + "]: " + this.message;
    }
}
const err = new AppError("bad", 7);
console.log(err instanceof AppError, err instanceof Error, err.message, err.code);
console.log(typeof err.describe === "function" ? err.describe() : "no method");
class Stack extends Array {
    peek() {
        return this[this.length - 1];
    }
}
const s = new Stack();
s.push(1, 2, 3);
console.log(s.length, s instanceof Stack, typeof s.peek === "function" ? s.peek() : "no method");
class Registry extends Map {
    register(k, v) {
        this.set(k, v);
        return this;
    }
}
const r = new Registry();
console.log(typeof r.register === "function" ? r.register("a", 1).get("a") : "no method", r instanceof Map);
