import { ALL, wide } from "../wide.mjs";

export default wide({ id: "babel-polyfill", group: "polyfills", name: "babel-polyfill", include: ALL, exclude: /^dist\/polyfill\.js$/ });
