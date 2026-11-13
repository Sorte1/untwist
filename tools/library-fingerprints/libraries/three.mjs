import { ALL, wide } from "../wide.mjs";

export default wide({ id: "three", name: "three", include: /^build\/three\.(module|min|module\.min)\.js$|^examples\/jsm\/(controls|loaders|postprocessing|shaders|math|utils|geometries|objects|lines|exporters|environments|curves|modifiers|interactive|csm|effects|animation|misc)\//, exclude: /^examples\/jsm\/libs\//, keepConcatenations: true });
