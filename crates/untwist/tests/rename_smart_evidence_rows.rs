use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

const PROPERTY_EVIDENCE: &str = "var state = { position: [1, 2], config: {} };
function Game(_0x1) {
  var _0x2;
  _0x2 = state.position;
  this.players = _0x1;
  state.config.dynamicHP = _0x2;
  var _0x3;
  _0x3 = state.position;
  _0x3 = state.config;
  return this;
}
var g = new Game(['a']);
console.log(g.players.length, state.config.dynamicHP.join());";

const CONSTRUCTOR_EVIDENCE: &str = "function WebSocket(url) { this.url = url; }
var THREE = { Vector3: function () { this.x = 0; } };
function setup() {
  var _0x1 = new WebSocket('ws://x');
  var _0x2;
  _0x2 = new THREE.Vector3();
  var _0x3 = new Float32Array(4);
  var _0x4 = new Map();
  var _0x5;
  _0x5 = new Promise(function (_0x6) { _0x6(1); });
  return [_0x1.url, _0x2.x, _0x3.length, _0x4.size, typeof _0x5.then].join();
}
console.log(setup());";

const DOM_EVIDENCE: &str = "var document = {
  getElementById: function (id) { return { id: id }; },
  querySelector: function (s) { return { id: s }; },
  createElement: function (t) { return { tag: t }; }
};
var localStorage = { data: { volume: '3' }, getItem: function (k) { return this.data[k] || null; }, setItem: function (k, v) { this.data[k] = v; } };
function ui() {
  var _0x1 = document.getElementById('chatList');
  var _0x2;
  _0x2 = document.querySelector('#menu-holder');
  var _0x3 = document.querySelector('.menu-item');
  var _0x4 = localStorage.getItem('volume');
  var _0x5 = 'x' + _0x4;
  localStorage.setItem('playerName', _0x5);
  var _0x6 = { getContext: function (k) { return k; } };
  var _0x7 = _0x6.getContext('webgl');
  return [_0x1.id, _0x2.id, _0x3.id, _0x4, localStorage.data.playerName, _0x7].join();
}
console.log(ui());";

const EVENT_EVIDENCE: &str = "var handlers = {};
var el = { addEventListener: function (k, f) { handlers[k] = f; } };
var socket = {};
function _0x1(_0x2) { console.log('down', _0x2.key); }
el.addEventListener('keydown', _0x1);
el.addEventListener('mousemove', function (_0x3) { console.log('move', _0x3.x); });
el.addEventListener('resize', function (_0x4) { console.log('resize', _0x4.type); });
socket.onmessage = function (_0x5) { console.log('msg', _0x5.data); };
function _0x6(_0x7) { console.log('close', _0x7.code); }
socket.onclose = _0x6;
handlers.keydown({ key: 'w' });
handlers.mousemove({ x: 3 });
handlers.resize({ type: 'resize' });
socket.onmessage({ data: 'hi' });
socket.onclose({ code: 1000 });";

const TIMER_EVIDENCE: &str = "var frames = 0;
var requestAnimationFrame = function (f) { if (frames < 2) { frames++; f(frames * 16); } };
function _0x1(_0x2) { console.log('frame', _0x2); requestAnimationFrame(_0x1); }
requestAnimationFrame(_0x1);
requestAnimationFrame(function (_0x3) { console.log('inline', _0x3); });
function _0x4() { console.log('later'); }
setTimeout(_0x4, 0);
var _0x5 = setInterval(function () { clearInterval(_0x5); console.log('tick'); }, 0);";

const FETCH_EVIDENCE: &str = "function fetch(url) {
  return Promise.resolve({ url: url, json: function () { return Promise.resolve({ n: 1 }); } });
}
fetch('/a').then(function (_0x1) { console.log(_0x1.url); return _0x1.json(); }).then(function (_0x2) { console.log(_0x2.n); });
fetch('/b').then(_0x3 => _0x3.json()).then(function (_0x4) { console.log(_0x4.n + 1); });";

const CONSTRUCTOR_FUNCTIONS: &str = "var game = {};
function _0x1(_0x2) { this.label = _0x2; }
game.Player = _0x1;
var _0x3 = function () { this.hp = 100; };
_0x3.prototype.hit = function () { this.hp--; return this.hp; };
var p = new _0x1('a');
var _0xc = new _0x3();
console.log(p.label, _0xc.hit(), new game.Player('b').label);";

const BOOLEAN_RETURNERS: &str = "function Player() {}
function _0x1(_0x2) { return _0x2 instanceof Player; }
function _0x3(_0x4) { return typeof _0x4 === 'string'; }
var _0x5 = function (_0x6) { if (!_0x6) { return false; } return _0x6.type === 'enemy'; };
var _0x7 = (_0x8, _0x9) => _0x8 === _0x9;
function _0xa(_0xb) { if (_0xb) { return 1; } return true; }
var checks = [_0x1, _0x3, _0x5, _0x7, _0xa];
var inputs = [new Player(), 'x', { type: 'enemy' }, 1, 0];
console.log(checks.map(function (check) { var input = inputs.shift(); return check(input, 1); }).join());";

const CALL_SITE_PARAMS: &str = "var player = { hp: 3 };
var world = { gravity: 9 };
world.gravity += Math.floor(Math.random() * 0);
function _0x1(_0x2, _0x3, _0x4) { return _0x2.hp + _0x3 + _0x4; }
function _0x5(_0x6) { return _0x6.hp; }
var _0x7 = function (_0x8) { return _0x8; };
console.log(_0x1(player, world.gravity, 1), _0x1(player, world.gravity, 2));
console.log(_0x5(player), _0x5({ hp: 1 }));
var hold = [_0x7];
console.log(_0x7(player).hp, hold.length);";

const SETTER_PARAMS: &str = "var store = {};
var settings = {
  volume: { set: function (_0x1) { store.volume = _0x1; } },
  get level() { return store.level; },
  set level(_0x2) { store.level = _0x2 + 1; }
};
class Box { set size(_0x3) { this._size = _0x3; } }
settings.volume.set(2);
settings.level = 4;
var b = new Box();
b.size = 7;
console.log(store.volume, settings.level, b._size);";

const SETTER_PARAM_COMPUTED: &str = "var store = {};
var settings = { volume: { set: function (_0x1) { store.volume = _0x1 * 2; } } };
settings.volume.set(2);
console.log(store.volume);";

const EVIDENCE_NAME_CAPTURED: &str = "var keyEvent = 'outer';
var el = { addEventListener: function (k, f) { f({ key: k }); } };
el.addEventListener('keyup', function (_0x1) { console.log(_0x1.key, keyEvent); });
var chatList = 'outer list';
var document = { getElementById: function (id) { return { id: id }; } };
function f() { var _0x2 = document.getElementById('chatList'); return _0x2.id + chatList; }
console.log(f());";

const VALUE_METHOD_SELF_READ: &str = "Number.prototype.countDecimals = function () {
  if (Math.floor(this.valueOf()) === this.valueOf()) { return 0; }
  return this.toString().split('.')[1].length || 0;
};
var _0x1 = [1.25, 3].map(function (_0x2) { return _0x2.countDecimals(); });
console.log(_0x1.join());";

const DECLARED_METHOD_SELF_READ: &str = "function Big(_0x1) { this.words = [_0x1]; }
function _0x5() { return '<Big ' + this.toString(16) + '>'; }
Big.prototype.toString = function (_0x2) { return this.words[0].toString(_0x2); };
Big.prototype.inspect = _0x5;
Big.prototype[Symbol.for('inspect')] = _0x5;
var _0x3 = [255, 16].map(function (_0x4) { return new Big(_0x4).inspect(); });
console.log(_0x3.join());";

const DECLARED_METHOD_ALSO_CALLED: &str = "function Big(_0x1) { this.words = [_0x1]; }
function _0x5() { return String(this.toString().indexOf('_0x6') >= 0); }
Big.prototype.inspect = _0x5;
var _0x3 = [255].map(function (_0x4) { return _0x5.call(function (_0x6) { return _0x6; }); });
console.log(_0x3.join());";

const WITH_SEES_EVIDENCE: &str = "var o = { keyEvent: 'from with', chatList: 'list' };
var el = { addEventListener: function (k, f) { f({ key: k }); } };
function w() { with (o) { el.addEventListener('keydown', function (_0x1) { console.log(keyEvent, _0x1.key); }); } }
w();";

const ELEMENT_EVIDENCE: &str =
    "var game = { players: [{ hp: 1 }, { hp: 2 }], enemies: [{ hp: 5 }] };
function total() {
  var _0x1 = 0;
  for (const _0x2 of game.players) { _0x1 += _0x2.hp; }
  var _0x3 = game.enemies[_0x1 - 3];
  return _0x1 + _0x3.hp;
}
console.log(total());";

const METHOD_CALL_SITES: &str = "function World() {
  this.gravity = 2;
  this.updateAim = function (_0x1, _0x2) { return _0x1.zoom * _0x2; };
}
var world = new World();
var player = { zoom: 3 };
var state = { progress: 4 };
var hud = { drawBar: function (_0x3) { return '#'.repeat(_0x3); } };
console.log(world.updateAim(player, state.progress), world.updateAim(player, state.progress), hud.drawBar(world.gravity));";

const HEX_TOKENS: &str = "(function () {
  var b475796ed633d5fd0485 = 'token';
  var f9e2c8d63a4b7128b031 = function (cb) { cb(1, 2); };
  var list = [7].map(function (c0ffee00c0ffee00c0ff) { return c0ffee00c0ffee00c0ff + 1; });
  var plus = function (c0ffee00c0ffee00c0ff) { return c0ffee00c0ffee00c0ff + 1; };
  var a1b2c3d4e5f6a7b8c9d0;
  if (Math.random() < 2) { a1b2c3d4e5f6a7b8c9d0 = 'second'; }
  var hasToken = typeof b475796ed633d5fd0485 != 'undefined';
  Promise.resolve(0).then(function (_0x9) {
    if (!_0x9) { _0x9 = hasToken ? a1b2c3d4e5f6a7b8c9d0 : undefined; }
    console.log(JSON.stringify({ bypassToken: _0x9 }));
  });
  var validationToken;
  validationToken = hasToken ? b475796ed633d5fd0485 : undefined;
  if (typeof f9e2c8d63a4b7128b031 == 'function') {
    f9e2c8d63a4b7128b031(function (a, b) { console.log(a + b, validationToken, list.join(), plus(list.length * 2)); });
  }
})();";

fn options(rename: RenameMode, unsafe_transforms: bool) -> Options {
    Options {
        rename,
        unsafe_transforms,
        unpack: false,
        jsx: false,
        ..Options::default()
    }
}

fn smart(source: &str) -> String {
    Pipeline::new(options(RenameMode::Smart, false))
        .run(source)
        .code()
        .to_string()
}

fn local_smart(source: &str) -> String {
    smart(&format!("(function () {{\n{source}\n}})();"))
}

fn unsafe_smart(source: &str) -> String {
    Pipeline::new(options(RenameMode::Smart, true))
        .run(source)
        .code()
        .to_string()
}

fn assert_has(code: &str, expected: &[&str]) {
    for want in expected {
        assert!(code.contains(want), "missing {want:?} in:\n{code}");
    }
}

#[test]
fn names_come_from_property_reads_and_writes() {
    let code = smart(PROPERTY_EVIDENCE);
    assert_has(
        &code,
        &[
            "position = state.position",
            "this.players = players",
            "function Game(players)",
        ],
    );
    assert!(!code.contains("dynamicHP = dynamicHP"), "{code}");
    assert!(
        code.contains("_0x3") || code.contains("v = state"),
        "{code}"
    );
}

#[test]
fn names_come_from_constructors() {
    let code = smart(CONSTRUCTOR_EVIDENCE);
    assert_has(
        &code,
        &[
            "socket = new WebSocket",
            "vector3 = new THREE.Vector3",
            "floats = new Float32Array",
            "map = new Map",
            "promise = new Promise",
            "function (resolve)",
        ],
    );
}

#[test]
fn names_come_from_dom_lookups_and_storage_keys() {
    let code = smart(DOM_EVIDENCE);
    assert_has(
        &code,
        &[
            "chatList = document.getElementById",
            "menuHolder = document.querySelector",
            "menuItem = document.querySelector",
            "volume = localStorage.getItem",
            "playerName = ",
            "gl = ",
        ],
    );
}

#[test]
fn event_handlers_and_their_parameters_are_named_by_event_type() {
    let code = local_smart(EVENT_EVIDENCE);
    assert_has(
        &code,
        &[
            "function onKeydown(keyEvent)",
            "function (mouseEvent)",
            "function (event)",
            "socket.onmessage = function (event)",
            "function onClose(event)",
        ],
    );
}

#[test]
fn timer_callbacks_are_named() {
    let code = local_smart(TIMER_EVIDENCE);
    assert_has(
        &code,
        &[
            "function onFrame(time)",
            "function (time)",
            "function onTimeout()",
            "timer = setInterval",
        ],
    );
}

#[test]
fn fetch_chains_name_the_response_and_the_body() {
    let code = smart(FETCH_EVIDENCE);
    assert_has(
        &code,
        &[
            "function (response)",
            "function (data)",
            "(response) => response.json()",
        ],
    );
    assert!(!code.contains("function (result)"), "{code}");
}

#[test]
fn constructor_functions_get_pascal_case_names() {
    let code = local_smart(CONSTRUCTOR_FUNCTIONS);
    assert_has(
        &code,
        &[
            "function Player(label)",
            "Ctor = function",
            "instance = new Ctor()",
        ],
    );
}

#[test]
fn boolean_returners_get_predicate_names() {
    let code = local_smart(BOOLEAN_RETURNERS);
    assert_has(
        &code,
        &[
            "function isPlayer(",
            "function isString(",
            "isEnemy = function",
            "predicate = (",
            "function fn(",
        ],
    );
}

#[test]
fn parameters_take_the_name_every_call_site_passes() {
    let code = local_smart(CALL_SITE_PARAMS);
    assert_has(
        &code,
        &[
            "(player, gravity, v)",
            "return player.hp + gravity",
            "function fn2(v)",
        ],
    );
    assert!(!code.contains("function (player)"), "{code}");
}

#[test]
fn setter_parameters_are_named_value_unless_written_somewhere_named() {
    let code = smart(SETTER_PARAMS);
    assert_has(
        &code,
        &[
            "set: function (volume)",
            "set level(value)",
            "set size(value)",
        ],
    );
    assert_has(&smart(SETTER_PARAM_COMPUTED), &["set: function (value)"]);
}

#[test]
fn an_evidence_name_an_outer_binding_holds_is_numbered() {
    let code = smart(EVIDENCE_NAME_CAPTURED);
    assert_has(&code, &["function (keyEvent2)", "chatList2 = document"]);
}

#[test]
fn a_value_method_reading_its_own_text_blocks_renaming_only_in_the_safe_lane() {
    let safe = smart(VALUE_METHOD_SELF_READ);
    assert!(safe.contains("_0x2"), "{safe}");
    let unsafe_lane = unsafe_smart(VALUE_METHOD_SELF_READ);
    assert!(!unsafe_lane.contains("_0x2"), "{unsafe_lane}");
}

#[test]
fn a_declared_function_installed_only_as_an_instance_method_reads_instances_in_the_unsafe_lane() {
    let safe = smart(DECLARED_METHOD_SELF_READ);
    assert!(safe.contains("_0x4"), "{safe}");
    let unsafe_lane = unsafe_smart(DECLARED_METHOD_SELF_READ);
    assert!(!unsafe_lane.contains("_0x4"), "{unsafe_lane}");
}

#[test]
fn a_declared_method_that_is_also_called_on_its_own_keeps_the_program_unrenamed() {
    let unsafe_lane = unsafe_smart(DECLARED_METHOD_ALSO_CALLED);
    assert!(unsafe_lane.contains("_0x4"), "{unsafe_lane}");
}

#[test]
fn an_exported_alias_names_its_local_binding() {
    let code =
        smart("var _0x1 = [1].map(function (_0x2) { return _0x2; });\nexport { _0x1 as values };");
    assert!(
        code.contains("values = [1].map") && code.contains("export { values }"),
        "{code}"
    );
}

#[test]
fn elements_of_a_plural_collection_take_the_singular_name() {
    let code = smart(ELEMENT_EVIDENCE);
    assert_has(
        &code,
        &[
            "for (const player of game.players)",
            "enemy = game.enemies[",
        ],
    );
}

#[test]
fn method_parameters_take_the_name_every_call_passes() {
    let code = smart(METHOD_CALL_SITES);
    assert_has(
        &code,
        &["function (player, progress)", "drawBar: function (gravity)"],
    );
}

#[test]
fn long_hex_tokens_are_machine_names_only_under_smart() {
    let code = smart(HEX_TOKENS);
    assert_has(
        &code,
        &[
            "validationToken2 = \"token\"",
            "callback = function",
            "typeof callback == \"function\"",
            "bypassToken = \"second\"",
            "hasToken ? bypassToken : undefined",
            "function (item)",
            "(c0ffee00c0ffee00c0ff) => {",
        ],
    );
    assert!(
        !code.contains("b475796ed633d5fd0485")
            && !code.contains("f9e2c8d63a4b7128b031")
            && !code.contains("a1b2c3d4e5f6a7b8c9d0"),
        "{code}"
    );
    for mode in [RenameMode::Simple, RenameMode::Unique] {
        let keep_human = Options {
            rename_all: false,
            ..options(mode, false)
        };
        let code = Pipeline::new(keep_human).run(HEX_TOKENS).code().to_string();
        assert!(
            code.contains("b475796ed633d5fd0485") && code.contains("f9e2c8d63a4b7128b031"),
            "{code}"
        );
    }
}

#[test]
fn node_agrees_on_every_new_evidence_kind_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = [
        ("property evidence", PROPERTY_EVIDENCE),
        ("constructor evidence", CONSTRUCTOR_EVIDENCE),
        ("dom evidence", DOM_EVIDENCE),
        ("event evidence", EVENT_EVIDENCE),
        ("timer evidence", TIMER_EVIDENCE),
        ("fetch evidence", FETCH_EVIDENCE),
        ("constructor functions", CONSTRUCTOR_FUNCTIONS),
        ("boolean returners", BOOLEAN_RETURNERS),
        ("call site params", CALL_SITE_PARAMS),
        ("setter params", SETTER_PARAMS),
        ("setter param computed", SETTER_PARAM_COMPUTED),
        ("evidence name captured", EVIDENCE_NAME_CAPTURED),
        ("value method self read", VALUE_METHOD_SELF_READ),
        ("declared method self read", DECLARED_METHOD_SELF_READ),
        ("declared method also called", DECLARED_METHOD_ALSO_CALLED),
        ("with sees evidence", WITH_SEES_EVIDENCE),
        ("element evidence", ELEMENT_EVIDENCE),
        ("method call sites", METHOD_CALL_SITES),
        ("hex tokens", HEX_TOKENS),
    ]
    .iter()
    .map(|(label, source)| (label.to_string(), source.to_string()))
    .collect();
    let modes: [Mode<'_>; 2] = [("smart", &smart), ("unsafe smart", &unsafe_smart)];
    let wrong = mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
