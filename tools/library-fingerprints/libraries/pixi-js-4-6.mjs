import { ALL, wide } from "../wide.mjs";

export default wide({ id: "pixi-js-4-6", group: "pixi-legacy", versions: /^[456]\./, name: "pixi.js", include: /^dist\/(browser\/)?pixi\.min\.js$/, keepConcatenations: true, minifiers: ["none", "terser"] });
