import { snippetCompletion, type Completion } from '@codemirror/autocomplete';

const TEMPLATES: readonly (readonly [string, string, string])[] = [
  ['function', 'function ${name}(${params}) {\n\t${}\n}', 'function declaration'],
  ['arrow', '(${params}) => {\n\t${}\n}', 'arrow function'],
  ['iife', '(function () {\n\t${}\n})();', 'immediately invoked function'],
  ['for', 'for (let ${index} = 0; ${index} < ${bound}; ${index}++) {\n\t${}\n}', 'counted loop'],
  ['forof', 'for (const ${name} of ${collection}) {\n\t${}\n}', 'for…of loop'],
  ['forin', 'for (const ${key} in ${object}) {\n\t${}\n}', 'for…in loop'],
  ['while', 'while (${}) {\n\t${}\n}', 'loop'],
  ['dowhile', 'do {\n\t${}\n} while (${});', 'loop'],
  ['if', 'if (${}) {\n\t${}\n}', 'block'],
  ['ifelse', 'if (${}) {\n\t${}\n} else {\n\t${}\n}', 'block'],
  ['switch', 'switch (${}) {\n\tcase ${}:\n\t\t${}\n\t\tbreak;\n\tdefault:\n\t\t${}\n}', 'block'],
  ['try', 'try {\n\t${}\n} catch (${error}) {\n\t${}\n}', 'block'],
  ['class', 'class ${name} {\n\tconstructor(${params}) {\n\t\t${}\n\t}\n}', 'class declaration'],
  ['log', 'console.log(${});', 'console.log'],
  ['require', 'const ${name} = require("${module}");', 'CommonJS import'],
  ['import', 'import { ${names} } from "${module}";', 'module import'],
];

const BOOST = -20;

export const jsSnippets: readonly Completion[] = TEMPLATES.map(([label, template, detail]) => snippetCompletion(template, { label, detail, type: 'snippet', boost: BOOST }));
