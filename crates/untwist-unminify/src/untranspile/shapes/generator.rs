use untwist_core::pass::{ExpressionKind, StatementKind};

use super::{Kind, Variant, TOKEN_CAP};

const TS: &str = "fixtures/untranspile/ts/generators.es5.js";
const TS_MIN: &str = "fixtures/untranspile/ts/generators.es5.min.js";
const TS_ASYNC: &str = "fixtures/untranspile/ts/async.es5.js";
const SWC: &str = "fixtures/untranspile/swc/generators.es5.js";
const SWC_MIN: &str = "fixtures/untranspile/swc/generators.es5.min.js";
const ESBUILD: &str = "fixtures/untranspile/esbuild/async.0.28.1-es2015.js";
const ESBUILD_MIN: &str = "fixtures/untranspile/esbuild/async.0.28.1-es2015.min.js";

const MACHINE_CAP: u32 = 600;

const DECLARATION: u8 = StatementKind::VariableDeclaration as u8;
const FUNCTION: u8 = StatementKind::FunctionDeclaration as u8;
const RETURN: u8 = StatementKind::ReturnStatement as u8;
const EXPRESSION_BODY: u8 = u8::MAX;

const LOGICAL: u8 = ExpressionKind::LogicalExpression as u8;
const NEW: u8 = ExpressionKind::NewExpression as u8;
const OBJECT: u8 = ExpressionKind::ObjectExpression as u8;
const NO_FOCUS: u8 = u8::MAX;

const fn variant(
    kind: Kind,
    hash: u64,
    (params, statements): (u8, u8),
    (lead, focus): (u8, u8),
    cap: u32,
    fixture: &'static str,
) -> Variant {
    Variant {
        kind,
        hash,
        params,
        statements,
        lead,
        focus,
        external: &[],
        fixture,
        cap,
    }
}

pub const VARIANTS: &[Variant] = &[
    variant(
        Kind::Generator,
        0x5260_13cb_8aae_e9f6,
        (2, 12),
        (DECLARATION, OBJECT),
        MACHINE_CAP,
        TS,
    ),
    variant(
        Kind::Generator,
        0x18da_f655_d5ec_6b71,
        (2, 12),
        (DECLARATION, OBJECT),
        MACHINE_CAP,
        TS_MIN,
    ),
    variant(
        Kind::Generator,
        0xdce4_57bc_e328_0e16,
        (2, 13),
        (DECLARATION, NO_FOCUS),
        MACHINE_CAP,
        SWC,
    ),
    variant(
        Kind::Generator,
        0xbf52_24a3_cd5b_e1a3,
        (2, 13),
        (DECLARATION, NO_FOCUS),
        MACHINE_CAP,
        SWC_MIN,
    ),
    variant(
        Kind::Values,
        0x9509_191f_a2f7_2087,
        (1, 6),
        (DECLARATION, LOGICAL),
        TOKEN_CAP,
        TS,
    ),
    variant(
        Kind::Values,
        0x4cab_915f_8019_b093,
        (1, 6),
        (DECLARATION, LOGICAL),
        TOKEN_CAP,
        TS_MIN,
    ),
    variant(
        Kind::Values,
        0x6179_ecdd_374d_f083,
        (1, 6),
        (DECLARATION, LOGICAL),
        TOKEN_CAP,
        SWC,
    ),
    variant(
        Kind::Values,
        0x6be7_5c94_b271_be47,
        (1, 6),
        (DECLARATION, LOGICAL),
        TOKEN_CAP,
        SWC_MIN,
    ),
    variant(
        Kind::Awaiter,
        0x299b_70ef_aede_5472,
        (4, 2),
        (FUNCTION, NO_FOCUS),
        TOKEN_CAP,
        TS_ASYNC,
    ),
    variant(
        Kind::Async,
        0xf430_4cf8_c3b4_ce1c,
        (3, 1),
        (RETURN, NEW),
        TOKEN_CAP,
        ESBUILD,
    ),
    variant(
        Kind::Async,
        0xfc5c_48b4_7d3e_488d,
        (3, 1),
        (EXPRESSION_BODY, NEW),
        TOKEN_CAP,
        ESBUILD_MIN,
    ),
];
