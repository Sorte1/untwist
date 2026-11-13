import { ALL, wide } from "../wide.mjs";

export default wide({ id: "pixi-js", group: "pixi", versions: /^[78]\./, name: "pixi.js", include: /^dist\/(browser\/)?pixi\.min\.js$/, keepConcatenations: true, minifiers: ["none", "terser"] });
