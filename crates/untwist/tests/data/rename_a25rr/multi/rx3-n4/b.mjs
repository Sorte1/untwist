const s = "typeof q + typeof zz";
console.log(Function("return eval")()(s));
export {};
