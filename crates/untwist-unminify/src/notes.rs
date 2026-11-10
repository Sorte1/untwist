use untwist_core::catalogue;

pub const LET_TO_CONST_SCOPE_HAZARD: &str = "let declarations kept: the program contains a direct \
     eval or a with statement, so a binding the scoping tables call constant may still be written";

pub const UNWRAP_COMMA_CALL_SCOPE_HAZARD: &str = "comma calls kept: the program contains a direct \
     eval or a with statement, so an identifier callee may resolve through an object environment \
     that supplies a receiver the unwrapped call would lose";

pub const UNTRANSPILE_SCOPE_HAZARD: &str = "transpiler helpers kept: the program contains a \
     direct eval or a with statement, so a helper binding may be rebound where the scoping tables \
     cannot see it";

pub const UNTRANSPILE_BUILTINS_WRITTEN: &str = "transpiler helpers kept: the program writes to a \
     built-in the helpers rely on (Object, Array, Function, Symbol, String, a prototype of one, or \
     the global object)";

pub const UNTRANSPILE_UNKNOWN_HELPER: &str = "a transpiler helper whose body matches no known \
     version was left in place";

pub const UNTRANSPILE_OBJECT_REST_KEPT: &str = "object rest helpers kept: rest copies own \
     enumerable keys through [[Set]] and a key list read before copying, which object rest syntax \
     does not";

pub const UNTRANSPILE_CLASS_PROTOTYPE_OPEN: &str = "transpiled classes kept: the class helper \
     leaves the prototype property writable, which a class does not";

pub const UNTRANSPILE_MACHINE_KEPT: &str = "a transpiled generator was kept: its state machine \
     does not map onto structured statements (or uses the machine state in a way the structuring \
     cannot prove)";

pub const UNTRANSPILE_ASYNC_KEPT: &str = "a transpiled async function was kept: its receiver, \
     arguments, parameters or bindings would change meaning as an async function";

pub const UNTRANSPILE_HELPER_UNTRUSTED: &str = "a transpiler helper was left in place: its \
     binding may hold something else when a call runs (a script-level helper the global object \
     can replace, a shared `this && this.x` definition outside a module, or a var helper whose \
     initialiser may not have run first)";

catalogue!(
    ALL_NOTES,
    LET_TO_CONST_SCOPE_HAZARD,
    UNWRAP_COMMA_CALL_SCOPE_HAZARD,
    UNTRANSPILE_SCOPE_HAZARD,
    UNTRANSPILE_BUILTINS_WRITTEN,
    UNTRANSPILE_UNKNOWN_HELPER,
    UNTRANSPILE_OBJECT_REST_KEPT,
    UNTRANSPILE_CLASS_PROTOTYPE_OPEN,
    UNTRANSPILE_MACHINE_KEPT,
    UNTRANSPILE_ASYNC_KEPT,
    UNTRANSPILE_HELPER_UNTRUSTED
);
