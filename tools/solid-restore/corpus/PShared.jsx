import { createSignal, createContext, useContext } from "solid-js";
console.log("module top 1");
const [g, setG] = createSignal((console.log("init g"), 1));
const Ctx = createContext((console.log("init ctx"), "d"));
const table = {};
table.k = (console.log("fill table"), "v");
console.log("module top 2");
function A() { return <b onClick={() => setG(g() + 1)}>{g()}{table.k}{useContext(Ctx)}</b>; }
function B() { table.k += "!"; return <i>{g() * 2}{table.k}</i>; }
export default function PShared() {
  return <div><A /><B /><Ctx.Provider value="p"><A /></Ctx.Provider><button onClick={() => setG(g() + 1)}>inc</button></div>;
}
