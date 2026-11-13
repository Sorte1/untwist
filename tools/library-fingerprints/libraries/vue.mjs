import { ALL, wide } from "../wide.mjs";

export default wide({ id: "vue", group: "frameworks", name: "vue", include: /^dist\/vue\.(runtime\.esm|runtime\.esm-bundler|runtime\.min|min|global\.prod|runtime\.global\.prod)\.js$/ });
