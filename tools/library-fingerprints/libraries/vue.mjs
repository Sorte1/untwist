import { ALL, wide } from "../wide.mjs";

export default wide({ id: "vue", name: "vue", include: /^dist\/[^/]+\.js$/, exclude: /compiler|server|ssr/ });
