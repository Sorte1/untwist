console.log("eval shared");
import("./index.js").then(m => console.log("bumped", m.bump()));
export const tag = "t";
