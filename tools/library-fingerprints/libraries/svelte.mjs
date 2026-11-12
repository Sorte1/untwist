import { ALL, wide } from "../wide.mjs";

export default wide({ id: "svelte", name: "svelte", include: ALL, exclude: /(^|\/)compiler[^/]*$|^compiler\// });
