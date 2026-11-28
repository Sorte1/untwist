import { copyFileSync, mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { encryptStrings, protectTarget } from "bla-agents";

const [caseJson, outDir] = process.argv.slice(2);
const spec = JSON.parse(caseJson);
const work = join("/tmp", "case");
const input = join(work, spec.name);
mkdirSync(work, { recursive: true });
copyFileSync(spec.input, input);

let protectInput = input;
let encrypted = 0;
if (spec.variant === "encrypt-strings") {
  const encDir = join("/tmp", "enc");
  const report = encryptStrings({ path: input, seed: spec.seed, outDir: encDir });
  if (report.status !== "ok") throw new Error(`encrypt-strings ${report.status}: ${report.error ?? ""}`);
  encrypted = report.totalEncrypted;
  protectInput = join(encDir, spec.name);
}

const options = {
  inputPath: protectInput,
  outputPath: join(outDir, spec.name),
  intensity: spec.intensity,
  seed: spec.seed,
  ...(spec.variant === "no-honey-map" ? { honeySourceMaps: false } : {}),
  ...(spec.variant === "llm-directives" ? { llmDirectives: true } : {}),
  ...(spec.variant === "self-seal" ? { selfSeal: true } : {}),
};
mkdirSync(dirname(options.outputPath), { recursive: true });
const result = protectTarget(options);
writeFileSync(
  join(outDir, "gen.json"),
  JSON.stringify({
    format: result.detectedFormat,
    intensity: result.intensity,
    encrypted,
    protectedSha256: result.protectedSha256,
    metrics: result.metrics,
    files: readdirSync(outDir).filter((file) => file !== "gen.json"),
  }),
);
process.stdout.write(`${basename(options.outputPath)} ${result.protectedSha256}\n`);
