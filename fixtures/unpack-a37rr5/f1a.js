if (typeof window === "undefined") { function g() { return "fn"; } }
if (typeof window !== "undefined") { var g; }
console.log(typeof g);
