use untwist_core::pass::{ExpressionKind, StatementKind};

use super::{Alias, Kind, Variant};

const BABEL: &str = "fixtures/untranspile/babel/destructure.7.29.js";
const BABEL_MIN: &str = "fixtures/untranspile/babel/destructure.7.29.min.js";
const BABEL_REST: &str = "fixtures/untranspile/babel/rest.7.29.js";
const SWC: &str = "fixtures/untranspile/swc/destructure.es5.js";
const SWC_MIN: &str = "fixtures/untranspile/swc/destructure.es5.min.js";
const SWC_REST: &str = "fixtures/untranspile/swc/rest.es5.js";
const SWC_REST_MIN: &str = "fixtures/untranspile/swc/rest.es5.min.js";
const TS_REST: &str = "fixtures/untranspile/ts/rest.es5.js";
const TS_REST_MIN: &str = "fixtures/untranspile/ts/rest.es5.min.js";
const ESBUILD_REST: &str = "fixtures/untranspile/esbuild/rest.0.28.1-es2015.js";

const IF: u8 = StatementKind::IfStatement as u8;
const RETURN: u8 = StatementKind::ReturnStatement as u8;
const THROW: u8 = StatementKind::ThrowStatement as u8;
const DECLARATION: u8 = StatementKind::VariableDeclaration as u8;

const CALL: u8 = ExpressionKind::CallExpression as u8;
const CONDITIONAL: u8 = ExpressionKind::ConditionalExpression as u8;
const LOGICAL: u8 = ExpressionKind::LogicalExpression as u8;
const NEW: u8 = ExpressionKind::NewExpression as u8;
const BINARY: u8 = ExpressionKind::BinaryExpression as u8;
const OBJECT: u8 = ExpressionKind::ObjectExpression as u8;

const SLICED_PARTS: &[Kind] = &[
    Kind::ArrayWithHoles,
    Kind::IterableToArrayLimit,
    Kind::UnsupportedIterableToArray,
    Kind::NonIterableRest,
];

const LOOSE: &[Kind] = &[Kind::ObjectWithoutPropertiesLoose];

const OBJ_REST_PARTS: &[Kind] = &[
    Kind::Alias(Alias::HasOwnProperty),
    Kind::Alias(Alias::GetOwnPropertySymbols),
    Kind::Alias(Alias::PropertyIsEnumerable),
];

#[allow(clippy::too_many_arguments)]
const fn variant(
    kind: Kind,
    hash: u128,
    (params, statements): (u8, u8),
    lead: u8,
    focus: u8,
    external: &'static [Kind],
    fixture: &'static str,
) -> Variant {
    Variant {
        kind,
        hash,
        params,
        statements,
        lead,
        focus,
        external,
        fixture,
        cap: super::TOKEN_CAP,
    }
}

pub const VARIANTS: &[Variant] = &[
    variant(
        Kind::ArrayWithHoles,
        0xdcc3_037d_bcca_0439_fad9_8bcb_b4b0_3ca3,
        (1, 1),
        IF,
        CALL,
        &[],
        BABEL,
    ),
    variant(
        Kind::IterableToArrayLimit,
        0x6dcc_f45a_c3b1_de70_763d_42ae_3a9f_67c6,
        (2, 2),
        DECLARATION,
        CONDITIONAL,
        &[],
        BABEL,
    ),
    variant(
        Kind::IterableToArrayLimit,
        0xe45f_68dc_a31d_11e1_9104_738c_87c8_5931,
        (2, 2),
        DECLARATION,
        CONDITIONAL,
        &[],
        BABEL_MIN,
    ),
    variant(
        Kind::IterableToArrayLimit,
        0xf6e2_9388_f626_d47b_2d99_4751_dca1_5298,
        (2, 9),
        DECLARATION,
        CONDITIONAL,
        &[],
        SWC,
    ),
    variant(
        Kind::IterableToArrayLimit,
        0x06bd_dab7_3232_c76c_b222_765f_6090_2193,
        (2, 2),
        DECLARATION,
        CONDITIONAL,
        &[],
        SWC_MIN,
    ),
    variant(
        Kind::NonIterableRest,
        0x58c2_d314_bf15_79a4_27cb_eb3b_4862_ad0b,
        (0, 1),
        THROW,
        NEW,
        &[],
        BABEL,
    ),
    variant(
        Kind::NonIterableRest,
        0x2bc3_9f9d_2726_c56a_c052_a76f_3f70_3412,
        (0, 1),
        THROW,
        NEW,
        &[],
        BABEL_MIN,
    ),
    variant(
        Kind::SlicedToArray,
        0x1a09_12b5_e13b_4e8c_dc32_8ff7_19e3_2f7b,
        (2, 1),
        RETURN,
        LOGICAL,
        SLICED_PARTS,
        BABEL,
    ),
    variant(
        Kind::ObjectWithoutProperties,
        0x5468_2284_e6ec_4c7e_b65a_02ee_48ac_e3ae,
        (2, 6),
        IF,
        BINARY,
        LOOSE,
        BABEL_REST,
    ),
    variant(
        Kind::ObjectWithoutProperties,
        0x61cc_2d08_cf92_2ca0_a408_83c1_54f7_da49,
        (2, 9),
        IF,
        BINARY,
        LOOSE,
        SWC_REST,
    ),
    variant(
        Kind::ObjectWithoutProperties,
        0xe845_42dc_9653_1499_9ef0_2c14_6067_0589,
        (2, 9),
        IF,
        BINARY,
        LOOSE,
        SWC_REST_MIN,
    ),
    variant(
        Kind::ObjectWithoutPropertiesLoose,
        0x3e44_7c9f_0518_b8f8_d2b0_ef5d_572f_21da,
        (2, 4),
        IF,
        BINARY,
        &[],
        BABEL_REST,
    ),
    variant(
        Kind::ObjectWithoutPropertiesLoose,
        0xb1d6_072e_74c6_2e4b_bd08_49e1_1c5b_ec4a,
        (2, 7),
        IF,
        BINARY,
        &[],
        SWC_REST,
    ),
    variant(
        Kind::ObjectWithoutPropertiesLoose,
        0xe61a_1009_669b_fcd2_ffc0_fcbc_c208_d5bb,
        (2, 7),
        IF,
        BINARY,
        &[],
        SWC_REST_MIN,
    ),
    variant(
        Kind::Rest,
        0x9971_f330_abf4_40b4_216d_309e_d44b_cf98,
        (2, 4),
        DECLARATION,
        OBJECT,
        &[],
        TS_REST,
    ),
    variant(
        Kind::Rest,
        0x26a2_272e_15b4_d6ab_b29c_bac2_897d_d8e6,
        (2, 4),
        DECLARATION,
        OBJECT,
        &[],
        TS_REST_MIN,
    ),
    variant(
        Kind::ObjRest,
        0x2b34_3383_62ba_8248_1bcc_b127_f2e1_9db3,
        (2, 4),
        DECLARATION,
        OBJECT,
        OBJ_REST_PARTS,
        ESBUILD_REST,
    ),
];
