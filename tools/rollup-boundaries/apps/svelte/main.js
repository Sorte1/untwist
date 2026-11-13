import { render } from "svelte/server";
import App from "./App.svelte";

const first = render(App, { props: { name: "untwist" } });
console.log(first.body.replace(/<!--[^>]*-->/g, ""));
const second = render(App, { props: { name: "rollup" } });
console.log(second.body.length > 0, second.head.length);
