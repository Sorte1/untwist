import { ALL, wide } from "../wide.mjs";

export default wide({ id: "svelte", group: "frameworks", name: "svelte", include: /^(internal|src\/internal|src\/runtime)\//, exclude: /(^|\/)(compiler|server)(\/|\.|$)/ });
