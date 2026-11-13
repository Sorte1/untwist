const helperTexts = [
  ["classCallCheck", /Cannot call a class as a function/g],
  ["inherits", /Super expression must either be null or a function/g],
  ["assertThisInitialized", /hasn't been initiali[sz]ed - super\(\) hasn't been called/g],
  ["possibleConstructorReturn", /Derived constructors may only return object or undefined/g],
  ["tsExtends", /Class extends value " \+ String\(b\) \+ " is not a constructor or null|Class extends value .{1,40} is not a constructor or null/g],
  ["tsGenerator", /Generator is already executing\./g],
  ["nonIterableSpread", /Invalid attempt to spread non-iterable instance/g],
  ["nonIterableRest", /Invalid attempt to destructure non-iterable instance/g],
  ["nonIterableForOf", /Invalid attempt to iterate non-iterable instance/g],
  ["tsValues", /Symbol\.iterator is not defined\./g],
  ["privateGet", /attempted to (?:get|set) private field on non-instance|Cannot read private member|Cannot write private member|Private element is not present on this object|Cannot (?:access|add) private|private field|Object must be an instance of class/gi],
  ["toPrimitive", /@@toPrimitive must return a primitive value\./g],
  ["regenerator", /try statement without catch or finally|Generator is already running|regeneratorRuntime|_regenerator\(\)|_regeneratorRuntime/g],
  ["asyncStep", /function asyncGeneratorStep|_asyncToGenerator|__awaiter|__async\b/g],
  ["jscomp", /\$jscomp\.(?:inherits|generator|asyncExecutePromiseGeneratorFunction|makeIterator|arrayFromIterator|arrayFromIterable|createTemplateTagFirstArg|construct|getRestArguments|objectCreate)/g],
];

const helperNames = [
  ["babelClass", /\b_(?:classCallCheck|createClass|inherits|callSuper|possibleConstructorReturn|getPrototypeOf|setPrototypeOf|assertThisInitialized|isNativeReflectConstruct|createSuper|wrapNativeSuper|get|superPropGet|superPropBase|superPropSet|set|defineProperty|defineProperties|classPrivateFieldGet2?|classPrivateFieldSet2?|classPrivateFieldInitSpec|classPrivateMethodInitSpec|classPrivateMethodGet|classStaticPrivateFieldSpecGet|classStaticPrivateMethodGet|classStaticPrivateFieldSpecSet|checkPrivateRedeclaration|classPrivateFieldLooseBase|classPrivateFieldLooseKey|assertClassBrand|classApplyDescriptorGet|classApplyDescriptorSet|classExtractFieldDescriptor|classCheckPrivateStaticAccess|classPrivateGetter|classPrivateSetter|toPropertyKey|toPrimitive|typeof|inheritsLoose|construct|isNativeFunction)\b/g],
  ["babelIter", /\b_(?:toConsumableArray|arrayWithoutHoles|iterableToArray|unsupportedIterableToArray|nonIterableSpread|arrayLikeToArray|slicedToArray|arrayWithHoles|iterableToArrayLimit|nonIterableRest|createForOfIteratorHelper(?:Loose)?|toArray|maybeArrayLike)\b/g],
  ["babelObject", /\b_(?:objectSpread2?|extends|objectWithoutProperties(?:Loose)?|objectDestructuringEmpty|ownKeys)\b/g],
  ["babelAsync", /\b_(?:asyncToGenerator|regeneratorRuntime|regenerator|awaitAsyncGenerator|wrapAsyncGenerator|asyncIterator|AsyncGenerator|asyncGeneratorDelegate|OverloadYield|regeneratorValues|regeneratorKeys|regeneratorAsync(?:Gen|Iterator)?|regeneratorDefine2?)\b/g],
  ["babelTemplate", /\b_taggedTemplateLiteral(?:Loose)?\b/g],
  ["tslib", /\b__(?:extends|assign|rest|awaiter|generator|values|read|spread|spreadArray|spreadArrays|makeTemplateObject|classPrivateFieldGet|classPrivateFieldSet|classPrivateFieldIn|await|asyncGenerator|asyncValues|asyncDelegator|setFunctionName|propKey|esDecorate|runInitializers|createBinding)\b/g],
  ["esbuild", /\b__(?:publicField|privateGet|privateSet|privateAdd|privateMethod|privateIn|privateWrapper|async|spreadValues|spreadProps|objRest|pow|forAwait|yieldStar|await|template|superGet|superSet|accessCheck|defNormalProp|defProp|defProps|getOwnPropSymbols|hasOwnProp|propIsEnum|knownSymbol|name)\b/g],
  ["swc", /\b_(?:class_call_check|create_class|inherits|create_super|call_super|possible_constructor_return|get_prototype_of|set_prototype_of|assert_this_initialized|is_native_reflect_construct|define_property|to_consumable_array|sliced_to_array|object_spread(?:_props)?|object_without_properties(?:_loose)?|async_to_generator|ts_generator|class_private_field_get|class_private_field_set|class_private_field_init|class_private_method_init|class_private_method_get|class_static_private_field_spec_get|tagged_template_literal|to_array|type_of|wrap_native_super|super_prop_base|get|create_for_of_iterator_helper_loose|instanceof|array_like_to_array|array_with_holes|array_without_holes|iterable_to_array|non_iterable_rest|non_iterable_spread|unsupported_iterable_to_array|async_generator|await_async_generator|wrap_async_generator|async_iterator|extends|class_apply_descriptor_get|class_apply_descriptor_set|class_extract_field_descriptor|check_private_redeclaration|class_check_private_static_access|class_static_private_method_get|class_private_field_loose_base|class_private_field_loose_key|construct|is_native_function|to_property_key|to_primitive|ts_values)\b/g],
];

const loweredForms = [
  ["optional", /(?:===|==) null \|\| [\w$.]+ (?:===|==) (?:void 0|undefined) \? (?:void 0|undefined) :|(?:===|==) null \? (?:void 0|undefined) :|null == [\w$.]+ \? (?:void 0|undefined) :|== null \? (?:void 0|undefined)|null==[\w$]+\?(?:void 0|undefined):|===null\|\|[\w$]+===(?:void 0|undefined)\?(?:void 0|undefined):|!= null \? [\w$]+[.[(]/g],
  ["nullish", /!== null && [\w$.]+ !== (?:void 0|undefined) \? [\w$.]+ :|!= null \? [\w$.]+ :|!==null&&[\w$]+!==(?:void 0|undefined)\?[\w$]+:|typeof [\w$.]+ !== "undefined" && [\w$.]+ !== null \?/g],
  ["pow", /\bMath\.pow\(/g],
  ["concat", /"[^"\n]*"\.concat\(|''\.concat\(|""\.concat\(/g],
  ["loop", /\b_loop\d*\b|\b_ret\d*\b/g],
  ["thisAlias", /\bvar _this\d* = this\b|\b_this\d* = this[;,]|\bvar _arguments\d* = arguments\b|_newArrowCheck/g],
  ["defaults", /arguments\.length > \d+ && arguments\[\d+\] !== undefined \? arguments\[\d+\] :|=== (?:void 0|undefined) \? [^:;\n]{1,40} : [\w$]+;|=== (?:void 0|undefined)\) \{ [\w$]+ = /g],
  ["restArgs", /for \(var _len\d* = arguments\.length|new Array\(_len\d*|Array\.prototype\.slice\.call\(arguments|\[\]\.slice\.call\(arguments|slice\.call\(arguments/g],
  ["templateObject", /_templateObject\d*\b|__makeTemplateObject|__template\(/g],
  ["classIife", /\/\*\* @class \*\/|\/\*#__PURE__\*\/function \(\)|\/\*#__PURE__\*\/\(?function \(/g],
  ["symbolIterator", /\[Symbol\.iterator\]\(\)[;,]|\btypeof Symbol === "function" && Symbol\.iterator|_iteratorNormalCompletion|_iteratorError|_iterator\d*\.s\(\)|\b_step\d*\b/g],
];

export const markerGroups = [...helperTexts, ...helperNames, ...loweredForms];

export function residue(code) {
  const out = {};
  let total = 0;
  for (const [name, re] of markerGroups) {
    const n = (code.match(re) || []).length;
    if (n) {
      out[name] = n;
      total += n;
    }
  }
  return { total, groups: out };
}

const id = String.raw`[\w$]+`;
const sig = (...parts) => parts.map((part) => new RegExp(part.replaceAll("ID", id), "m"));

export const signatures = {
  "arrow-this": sig(String.raw`\(ID\) => \{`, String.raw`=> this\.ID`),
  "async-await": sig(String.raw`async function ID\(`, String.raw`async \(?ID\)? => `, String.raw`\bawait `),
  "async-generators": sig(String.raw`async function\s*\*\s*ID\(`, String.raw`for await \(`),
  "block-scoping": sig(String.raw`\b(?:let|const) ID = `),
  "class-basic": sig(String.raw`class(?: ID)? \{`, String.raw`get ID\(\) \{`, String.raw`static ID\(\) \{`),
  "class-builtins": sig(String.raw`class ID extends Error`, String.raw`class ID extends Array`, String.raw`class ID extends Map`),
  "class-extends": sig(String.raw`class ID extends ID`, String.raw`super\.ID\(`, String.raw`super\(`),
  "class-fields": sig(String.raw`^\s+ID = 0;`, String.raw`^\s+static ID = 0;`),
  "class-private": sig(String.raw`#ID = 0`, String.raw`#ID\(`),
  "class-static-block": sig(String.raw`static \{`),
  destructuring: sig(String.raw`\[ID, , ID = 10, \.\.\.ID\]`, String.raw`\{ ID, y: ID = 2`),
  exponent: sig(String.raw`\*\* ?-?\(?\d`, String.raw`\*\*=`),
  "for-of": sig(String.raw`for \((?:const|let|var) (?:ID|\[[^\]]*\]) of `),
  generators: sig(String.raw`function\s*\*\s*ID\(`, String.raw`yield\* `),
  "logical-assignment": sig(String.raw`\|\|=`, String.raw`&&=`, String.raw`\?\?=`),
  nullish: sig(String.raw`\?\?`),
  "numeric-separators": [],
  "object-literals": sig(String.raw`super\.ID\(`),
  "object-spread-rest": sig(String.raw`\{\s*\.\.\.ID,\s*\.\.\.ID,`, String.raw`\.\.\.ID\s*\}\s*=`),
  "optional-chaining": sig(String.raw`\?\.ID`, String.raw`\?\.\(`, String.raw`\?\.\[`),
  params: sig(String.raw`ID = "anon"`, String.raw`\.\.\.ID\) \{`),
  spread: sig(String.raw`\[\.\.\.ID, \.\.\.ID, 5\]`, String.raw`ID\(\.\.\.ID, \.\.\.ID\)`),
  "template-literals": sig("`hello \\$\\{", "ID`a\\$\\{1\\}b"),
  classes: sig(String.raw`class ID extends ID`),
  soaks: sig(String.raw`\?\.ID`),
  splats: sig(String.raw`\.\.\.ID\)`),
};

export function restoredParts(feature, code) {
  const list = signatures[feature] ?? [];
  return { matched: list.filter((re) => re.test(code)).length, of: list.length };
}
