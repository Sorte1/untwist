export const GLOBALS = `
console | The debugging console: log, warn, error, table and timers.
Math | Mathematical constants and functions.
JSON | Parses and serialises JSON text.
Reflect | Functions that perform the default action of each object operation.
Intl | Locale-aware formatting of numbers, dates and text.
Atomics | Atomic operations on shared memory.
globalThis: object | The global object, in every environment.
window: Window | The browser's global object.
self: object | The global object of a window or a worker.
document: Document | The page's DOM document.
navigator: Navigator | The browser and the device it runs on.
location: Location | The page's URL.
module: object | The CommonJS module record; module.exports is what require returns.
exports: object | The CommonJS exports object of this module.
require(id): any | Loads a CommonJS module and returns its exports.
undefined: undefined | The value of a missing property or an unassigned variable.
NaN: number | Not a number.
Infinity: number | Positive infinity.
parseInt(string, radix?): number | Parses a string into an integer in the given radix (2 to 36).
parseFloat(string): number | Parses a string into a floating-point number.
isNaN(value): boolean | Whether the value, converted to a number, is NaN.
isFinite(value): boolean | Whether the value, converted to a number, is finite.
encodeURIComponent(component): string | Percent-encodes a URI component, including reserved characters.
decodeURIComponent(component): string | Decodes a percent-encoded URI component.
encodeURI(uri): string | Percent-encodes a full URI, keeping its reserved characters.
decodeURI(uri): string | Decodes a URI encoded with encodeURI.
escape(string): string | Legacy percent-encoding; prefer encodeURIComponent.
unescape(string): string | Decodes a string encoded with escape.
atob(data): string | Decodes a base64 string into a binary string.
btoa(data): string | Encodes a binary string as base64.
eval(code): any | Runs a string as JavaScript in the current scope.
setTimeout(handler, timeout?, ...args): number | Calls the handler once after the delay in milliseconds.
setInterval(handler, timeout?, ...args): number | Calls the handler repeatedly, every delay milliseconds.
clearTimeout(id): void | Cancels a timeout set with setTimeout.
clearInterval(id): void | Cancels an interval set with setInterval.
queueMicrotask(callback): void | Runs the callback after the current task, before the next one.
structuredClone(value, options?): any | Deep-copies a value with the structured clone algorithm.
fetch(input, init?): Promise<Response> | Starts an HTTP request and resolves to its response.
String(value?): string | Converts a value to a string.
Number(value?): number | Converts a value to a number.
Boolean(value?): boolean | Converts a value to true or false.
Symbol(description?): symbol | Creates a new unique symbol.
BigInt(value): bigint | Converts a value to a BigInt.
new Object(value?) | Wraps a value in an object, or creates an empty one.
new Array(...items) | Creates an array of the items, or of the given length.
new Function(...args, body) | Creates a function from source text.
new Map(entries?) | A keyed collection that remembers insertion order.
new Set(values?) | A collection of unique values in insertion order.
new WeakMap(entries?) | A map whose object keys are held weakly.
new WeakSet(values?) | A set whose object values are held weakly.
new Promise(executor) | A value that settles later; executor receives resolve and reject.
new Date(value?) | A moment in time, from a timestamp, a string or the parts.
new RegExp(pattern, flags?) | A regular expression.
new Error(message?, options?) | An error with a message and a stack trace.
new TypeError(message?) | An error for a value of the wrong type.
new RangeError(message?) | An error for a value outside its range.
new SyntaxError(message?) | An error for invalid syntax.
new Proxy(target, handler) | Wraps an object and intercepts its operations.
new ArrayBuffer(length) | A fixed-length raw binary buffer.
new Uint8Array(source) | An array of 8-bit unsigned integers.
new DataView(buffer, byteOffset?, byteLength?) | Reads and writes numbers of any width in a buffer.
new TextEncoder() | Encodes strings as UTF-8 bytes.
new TextDecoder(label?, options?) | Decodes bytes in an encoding into a string.
new URL(url, base?) | A parsed URL.
new URLSearchParams(init?) | The query string of a URL as key and value pairs.
`;

export const NAMESPACES: Readonly<Record<string, string>> = {
  console: `
log(...data): void | Prints a message.
info(...data): void | Prints an informational message.
warn(...data): void | Prints a warning.
error(...data): void | Prints an error.
debug(...data): void | Prints a debug message.
table(data, columns?): void | Prints an array or object as a table.
dir(item, options?): void | Prints an object's properties.
trace(...data): void | Prints the message with a stack trace.
assert(condition?, ...data): void | Prints an error when the condition is false.
time(label?): void | Starts a timer.
timeEnd(label?): void | Stops a timer and prints the elapsed time.
group(...data): void | Starts an indented group of messages.
groupEnd(): void | Ends the current group.
count(label?): void | Prints how many times it was called with the label.
clear(): void | Clears the console.
`,
  Math: `
PI: number | The ratio of a circle's circumference to its diameter.
E: number | Euler's number, the base of natural logarithms.
abs(x): number | The absolute value.
ceil(x): number | Rounds up to the next integer.
floor(x): number | Rounds down to the previous integer.
round(x): number | Rounds to the nearest integer.
trunc(x): number | Drops the fractional part.
sign(x): number | 1, -1 or 0, by the sign of x.
max(...values): number | The largest of the values.
min(...values): number | The smallest of the values.
pow(x, y): number | x to the power y.
sqrt(x): number | The square root.
cbrt(x): number | The cube root.
random(): number | A pseudo-random number in [0, 1).
log(x): number | The natural logarithm.
log2(x): number | The base-2 logarithm.
log10(x): number | The base-10 logarithm.
exp(x): number | e to the power x.
sin(x): number | The sine of an angle in radians.
cos(x): number | The cosine of an angle in radians.
tan(x): number | The tangent of an angle in radians.
atan2(y, x): number | The angle of the point (x, y) from the x axis.
hypot(...values): number | The square root of the sum of squares.
imul(a, b): number | 32-bit integer multiplication.
clz32(x): number | Leading zero bits in the 32-bit form of x.
`,
  JSON: `
parse(text, reviver?): any | Parses JSON text into a value.
stringify(value, replacer?, space?): string | Serialises a value as JSON text.
`,
  Object: `
keys(object): string[] | The object's own enumerable string keys.
values(object): any[] | The object's own enumerable property values.
entries(object): [string, any][] | The object's own enumerable key and value pairs.
fromEntries(entries): object | Builds an object from key and value pairs.
assign(target, ...sources): object | Copies own enumerable properties onto the target.
create(proto, properties?): object | Creates an object with the given prototype.
freeze(object): object | Makes an object immutable.
isFrozen(object): boolean | Whether the object is frozen.
defineProperty(object, key, descriptor): object | Defines or changes one property.
defineProperties(object, descriptors): object | Defines or changes several properties.
getOwnPropertyNames(object): string[] | Every own string key, enumerable or not.
getOwnPropertyDescriptor(object, key): PropertyDescriptor | The descriptor of an own property.
getPrototypeOf(object): object | The object's prototype.
setPrototypeOf(object, proto): object | Sets the object's prototype.
hasOwn(object, key): boolean | Whether the object has the key as an own property.
is(a, b): boolean | SameValue equality: like === but NaN equals NaN and +0 differs from -0.
`,
  Array: `
isArray(value): boolean | Whether the value is an array.
from(items, mapFn?, thisArg?): any[] | Creates an array from an iterable or array-like.
of(...items): any[] | Creates an array of the arguments.
`,
  String: `
fromCharCode(...codes): string | Creates a string from UTF-16 code units.
fromCodePoint(...codePoints): string | Creates a string from code points.
raw(template, ...substitutions): string | The raw text of a template literal.
`,
  Number: `
isInteger(value): boolean | Whether the value is an integer number.
isSafeInteger(value): boolean | Whether the value is an integer exactly representable as a double.
isFinite(value): boolean | Whether the value is a finite number, without conversion.
isNaN(value): boolean | Whether the value is NaN, without conversion.
parseFloat(string): number | Parses a string into a floating-point number.
parseInt(string, radix?): number | Parses a string into an integer.
MAX_SAFE_INTEGER: number | 2^53 - 1.
MIN_SAFE_INTEGER: number | -(2^53 - 1).
EPSILON: number | The gap between 1 and the next double.
`,
  Promise: `
resolve(value?): Promise | A promise fulfilled with the value.
reject(reason?): Promise | A promise rejected with the reason.
all(promises): Promise | Fulfils with every result, or rejects with the first rejection.
allSettled(promises): Promise | Fulfils with the outcome of every promise.
any(promises): Promise | Fulfils with the first fulfilment.
race(promises): Promise | Settles like the first promise to settle.
`,
  Reflect: `
apply(target, thisArgument, args): any | Calls a function with a this value and arguments.
construct(target, args, newTarget?): object | Calls a constructor with new.
get(target, key, receiver?): any | Reads a property.
set(target, key, value, receiver?): boolean | Writes a property.
has(target, key): boolean | The in operator as a function.
ownKeys(target): (string | symbol)[] | Every own key, strings and symbols.
defineProperty(target, key, descriptor): boolean | Defines a property and reports success.
deleteProperty(target, key): boolean | The delete operator as a function.
getPrototypeOf(target): object | The object's prototype.
`,
  Symbol: `
iterator: symbol | The key of an object's default iterator.
asyncIterator: symbol | The key of an object's default async iterator.
toPrimitive: symbol | The key of an object's conversion to a primitive.
for(key): symbol | The shared symbol registered under the key.
`,
  Date: `
now(): number | Milliseconds since the Unix epoch.
parse(text): number | Parses a date string into a timestamp.
UTC(year, monthIndex?, day?, hours?, minutes?, seconds?, ms?): number | A timestamp from UTC parts.
`,
  document: `
getElementById(id): Element | The element with the id.
querySelector(selectors): Element | The first element matching the CSS selectors.
querySelectorAll(selectors): NodeList | Every element matching the CSS selectors.
createElement(tagName, options?): Element | Creates an element.
createTextNode(data): Text | Creates a text node.
addEventListener(type, listener, options?): void | Listens for an event.
body: HTMLElement | The body element.
head: HTMLElement | The head element.
cookie: string | The page's cookies as one string.
title: string | The page title.
`,
  module: `
exports: object | What require returns for this module.
`,
};

export const PROTOTYPES: Readonly<Record<string, string>> = {
  object: `
hasOwnProperty(key): boolean | Whether the object has the key as an own property.
toString(): string | The object as a string.
valueOf(): any | The object's primitive value.
isPrototypeOf(object): boolean | Whether this object is in the other's prototype chain.
propertyIsEnumerable(key): boolean | Whether an own property is enumerable.
`,
  string: `
length: number | The number of UTF-16 code units.
charAt(index): string | The character at the index.
charCodeAt(index): number | The UTF-16 code unit at the index.
codePointAt(index): number | The code point that starts at the index.
at(index): string | The character at the index; negative counts from the end.
indexOf(search, position?): number | The first index of the search string, or -1.
lastIndexOf(search, position?): number | The last index of the search string, or -1.
includes(search, position?): boolean | Whether the string contains the search string.
startsWith(search, position?): boolean | Whether the string starts with the search string.
endsWith(search, endPosition?): boolean | Whether the string ends with the search string.
slice(start?, end?): string | The part from start up to end; negative counts from the end.
substring(start, end?): string | The part between two indexes.
substr(start, length?): string | Legacy: length characters from start.
split(separator, limit?): string[] | Splits the string at each separator.
replace(pattern, replacement): string | Replaces the first match, or every match of a global regexp.
replaceAll(pattern, replacement): string | Replaces every match.
match(regexp): RegExpMatchArray | Matches against a regular expression.
matchAll(regexp): Iterator | Every match of a global regular expression.
search(regexp): number | The index of the first match, or -1.
toUpperCase(): string | The string in upper case.
toLowerCase(): string | The string in lower case.
trim(): string | Drops whitespace at both ends.
trimStart(): string | Drops leading whitespace.
trimEnd(): string | Drops trailing whitespace.
padStart(length, fill?): string | Pads the start up to the length.
padEnd(length, fill?): string | Pads the end up to the length.
repeat(count): string | The string repeated count times.
concat(...strings): string | The strings appended.
normalize(form?): string | The Unicode normal form.
localeCompare(other, locales?, options?): number | Compares in the locale's sort order.
`,
  array: `
length: number | The number of elements.
push(...items): number | Appends items and returns the new length.
pop(): any | Removes and returns the last element.
shift(): any | Removes and returns the first element.
unshift(...items): number | Prepends items and returns the new length.
slice(start?, end?): any[] | A shallow copy of a part.
splice(start, deleteCount?, ...items): any[] | Removes and inserts elements in place.
concat(...items): any[] | A new array with the items appended.
join(separator?): string | The elements joined into a string.
reverse(): any[] | Reverses the array in place.
sort(compareFn?): any[] | Sorts the array in place.
indexOf(item, fromIndex?): number | The first index of the item, or -1.
lastIndexOf(item, fromIndex?): number | The last index of the item, or -1.
includes(item, fromIndex?): boolean | Whether the array contains the item.
find(predicate, thisArg?): any | The first element the predicate accepts.
findIndex(predicate, thisArg?): number | The index of the first element the predicate accepts.
findLast(predicate, thisArg?): any | The last element the predicate accepts.
filter(predicate, thisArg?): any[] | The elements the predicate accepts.
map(callbackfn, thisArg?): any[] | Each element passed through the callback.
forEach(callbackfn, thisArg?): void | Calls the callback for each element.
reduce(callbackfn, initialValue?): any | Folds the elements from the left.
reduceRight(callbackfn, initialValue?): any | Folds the elements from the right.
some(predicate, thisArg?): boolean | Whether any element passes.
every(predicate, thisArg?): boolean | Whether every element passes.
flat(depth?): any[] | Flattens nested arrays.
flatMap(callbackfn, thisArg?): any[] | Maps, then flattens one level.
fill(value, start?, end?): any[] | Fills a range with the value.
at(index): any | The element at the index; negative counts from the end.
keys(): Iterator | The indexes.
values(): Iterator | The elements.
entries(): Iterator | Index and element pairs.
toSorted(compareFn?): any[] | A sorted copy.
toReversed(): any[] | A reversed copy.
`,
  number: `
toFixed(digits?): string | The number with a fixed count of decimals.
toPrecision(precision?): string | The number with a count of significant digits.
toString(radix?): string | The number in a radix from 2 to 36.
`,
  regexp: `
test(string): boolean | Whether the string matches.
exec(string): RegExpExecArray | The next match with its groups, or null.
source: string | The pattern text.
flags: string | The flags.
lastIndex: number | Where the next global or sticky match starts.
`,
  function: `
call(thisArg, ...args): any | Calls the function with a this value and arguments.
apply(thisArg, args?): any | Calls the function with a this value and an array of arguments.
bind(thisArg, ...args): Function | A copy with this and leading arguments fixed.
name: string | The function's name.
length: number | The number of declared parameters.
prototype: object | The prototype given to instances made with new.
`,
  Map: `
get(key): any | The value for the key.
set(key, value): Map | Sets the value for the key.
has(key): boolean | Whether the key is present.
delete(key): boolean | Removes the key.
clear(): void | Removes every entry.
size: number | The number of entries.
keys(): Iterator | The keys in insertion order.
values(): Iterator | The values in insertion order.
entries(): Iterator | Key and value pairs in insertion order.
forEach(callbackfn, thisArg?): void | Calls the callback for each entry.
`,
  Set: `
add(value): Set | Adds the value.
has(value): boolean | Whether the value is present.
delete(value): boolean | Removes the value.
clear(): void | Removes every value.
size: number | The number of values.
values(): Iterator | The values in insertion order.
forEach(callbackfn, thisArg?): void | Calls the callback for each value.
`,
  Promise: `
then(onfulfilled?, onrejected?): Promise | Runs a callback once the promise settles.
catch(onrejected?): Promise | Runs a callback if the promise rejects.
finally(onfinally?): Promise | Runs a callback once the promise settles either way.
`,
  Date: `
getTime(): number | Milliseconds since the Unix epoch.
toISOString(): string | The date in ISO 8601 form, in UTC.
getFullYear(): number | The year, in local time.
getMonth(): number | The month from 0 to 11, in local time.
getDate(): number | The day of the month, in local time.
getDay(): number | The day of the week from 0 (Sunday), in local time.
getHours(): number | The hours, in local time.
getMinutes(): number | The minutes, in local time.
getSeconds(): number | The seconds, in local time.
toLocaleString(locales?, options?): string | The date and time in the locale's format.
`,
  Error: `
message: string | The error message.
name: string | The error's name, such as TypeError.
stack: string | The stack trace, where the engine provides one.
`,
};
