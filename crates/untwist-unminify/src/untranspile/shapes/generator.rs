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
    hash: u128,
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
        0xf6ca_29ea_f826_2579_8a37_62fc_f84e_4fa4,
        (2, 12),
        (DECLARATION, OBJECT),
        MACHINE_CAP,
        TS,
    ),
    variant(
        Kind::Generator,
        0x0ea8_07b2_eba3_0b8b_4dc0_db12_173a_e7f6,
        (2, 12),
        (DECLARATION, OBJECT),
        MACHINE_CAP,
        TS_MIN,
    ),
    variant(
        Kind::Generator,
        0x0a89_df6c_4e1f_2ee4_2dca_f599_363f_6929,
        (2, 13),
        (DECLARATION, NO_FOCUS),
        MACHINE_CAP,
        SWC,
    ),
    variant(
        Kind::Generator,
        0x290a_da09_7513_84ad_cf05_c1b0_12bf_5556,
        (2, 13),
        (DECLARATION, NO_FOCUS),
        MACHINE_CAP,
        SWC_MIN,
    ),
    variant(
        Kind::Values,
        0xf4ef_28f5_dbb4_e7f1_624f_ba3f_3915_cc0c,
        (1, 6),
        (DECLARATION, LOGICAL),
        TOKEN_CAP,
        TS,
    ),
    variant(
        Kind::Values,
        0xecfa_fe6f_13ec_d8e2_ed45_6626_b086_97f6,
        (1, 6),
        (DECLARATION, LOGICAL),
        TOKEN_CAP,
        TS_MIN,
    ),
    variant(
        Kind::Values,
        0x42ba_0d59_e284_36ef_331f_002d_7467_aa04,
        (1, 6),
        (DECLARATION, LOGICAL),
        TOKEN_CAP,
        SWC,
    ),
    variant(
        Kind::Values,
        0xbf11_5373_99a4_8789_fa5c_6c20_344b_aff4,
        (1, 6),
        (DECLARATION, LOGICAL),
        TOKEN_CAP,
        SWC_MIN,
    ),
    variant(
        Kind::Awaiter,
        0x50a9_aa1e_a802_84c4_e40e_ad0d_a5d0_d4c1,
        (4, 2),
        (FUNCTION, NO_FOCUS),
        TOKEN_CAP,
        TS_ASYNC,
    ),
    variant(
        Kind::Async,
        0xf9c6_035f_d144_e3ad_6586_00ed_1ae2_9419,
        (3, 1),
        (RETURN, NEW),
        TOKEN_CAP,
        ESBUILD,
    ),
    variant(
        Kind::Async,
        0x5940_bc0b_377f_d284_9248_898f_4aa8_60ba,
        (3, 1),
        (EXPRESSION_BODY, NEW),
        TOKEN_CAP,
        ESBUILD_MIN,
    ),
];
