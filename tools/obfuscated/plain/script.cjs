var PRECISION = 3;
var host = eval("require;");

function add(a, b) {
  return a + b;
}

function scale(value, factor) {
  return Number((value * (factor === undefined ? 1.5 : factor)).toFixed(PRECISION));
}

function shout(text) {
  return text.split(" ").map(function (word) { return word.toUpperCase(); }).join("! ") + "!";
}

function Greeter(name) {
  this.name = name;
  this.greeting = "welcome back";
}

Greeter.prototype.greet = function () {
  return this.greeting + ", " + this.name + " (" + this.name.length + " chars)";
};

function createStore(initial) {
  var state = Object.assign({}, initial);
  var log = [];
  return {
    get: function (key) {
      return Object.prototype.hasOwnProperty.call(state, key) ? state[key] : undefined;
    },
    update: function (key, value) {
      log.push(key + ":" + state[key] + "->" + value);
      state[key] = value;
    },
    snapshot: function () {
      return Object.assign({}, state);
    },
    history: function () {
      return log.slice();
    },
  };
}

function pad(text, width, right) {
  var fill = " ".repeat(Math.max(0, width - text.length));
  return right ? fill + text : text + fill;
}

function table(record) {
  return Object.keys(record)
    .map(function (key) { return pad(key, 8) + "= " + String(record[key]); })
    .join("\n");
}

function Emitter() {
  this.handlers = {};
}

Emitter.prototype.on = function (name, handler) {
  (this.handlers[name] = this.handlers[name] || []).push(handler);
  return this;
};

Emitter.prototype.emit = function (name, value) {
  (this.handlers[name] || []).forEach(function (handler) { handler(value); });
  return name in this.handlers;
};

function checksum(text) {
  var hash = 7;
  for (var i = 0; i < text.length; i++) hash = (hash * 31 + text.charCodeAt(i)) % 1000003;
  return "checksum " + hash;
}

var greeter = new Greeter("player");
console.log(greeter.greet());
console.log(shout("ready set go"));
console.log("sum", add(2, 3), "scaled", scale(7), "precision", PRECISION);
var store = createStore({ score: 0, lives: 3, name: "anon" });
var events = new Emitter();
events.on("hit", function (points) { store.update("score", store.get("score") + points); });
events.on("hit", function () { console.log("hit registered"); });
events.emit("hit", 25);
events.emit("hit", 40);
store.update("lives", store.get("lives") - 1);
console.log(table(store.snapshot()));
console.log(pad("left", 10) + "|" + pad("right", 10, true) + "|");
console.log("legacy module object", "host require is a " + typeof host, checksum("obfuscated webpack fixture"));
console.log("keys", Object.keys(store.snapshot()).join(","), "history", store.history().length);
