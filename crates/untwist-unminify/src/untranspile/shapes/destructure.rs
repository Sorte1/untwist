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
    hash: u64,
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
        0x09cc_ded2_726b_1002,
        (1, 1),
        IF,
        CALL,
        &[],
        BABEL,
    ),
    variant(
        Kind::IterableToArrayLimit,
        0xa904_84c9_dc94_4d18,
        (2, 2),
        DECLARATION,
        CONDITIONAL,
        &[],
        BABEL,
    ),
    variant(
        Kind::IterableToArrayLimit,
        0xd915_26c8_aeb0_138e,
        (2, 2),
        DECLARATION,
        CONDITIONAL,
        &[],
        BABEL_MIN,
    ),
    variant(
        Kind::IterableToArrayLimit,
        0x8138_7451_2162_ecd5,
        (2, 9),
        DECLARATION,
        CONDITIONAL,
        &[],
        SWC,
    ),
    variant(
        Kind::IterableToArrayLimit,
        0x5120_d98e_d5f2_064c,
        (2, 2),
        DECLARATION,
        CONDITIONAL,
        &[],
        SWC_MIN,
    ),
    variant(
        Kind::NonIterableRest,
        0x6bb6_67b2_9a4b_9f96,
        (0, 1),
        THROW,
        NEW,
        &[],
        BABEL,
    ),
    variant(
        Kind::NonIterableRest,
        0x35bf_3bc6_8f1e_3efb,
        (0, 1),
        THROW,
        NEW,
        &[],
        BABEL_MIN,
    ),
    variant(
        Kind::SlicedToArray,
        0x5ae9_6530_2a20_e940,
        (2, 1),
        RETURN,
        LOGICAL,
        SLICED_PARTS,
        BABEL,
    ),
    variant(
        Kind::ObjectWithoutProperties,
        0xe707_d5c1_e9bd_bf4c,
        (2, 6),
        IF,
        BINARY,
        LOOSE,
        BABEL_REST,
    ),
    variant(
        Kind::ObjectWithoutProperties,
        0x93f2_3ed7_de6f_8fc4,
        (2, 9),
        IF,
        BINARY,
        LOOSE,
        SWC_REST,
    ),
    variant(
        Kind::ObjectWithoutProperties,
        0xd9ae_8e1d_81f3_7cfe,
        (2, 9),
        IF,
        BINARY,
        LOOSE,
        SWC_REST_MIN,
    ),
    variant(
        Kind::ObjectWithoutPropertiesLoose,
        0x6cb5_5b95_65c7_b132,
        (2, 4),
        IF,
        BINARY,
        &[],
        BABEL_REST,
    ),
    variant(
        Kind::ObjectWithoutPropertiesLoose,
        0x4bad_167a_4634_aced,
        (2, 7),
        IF,
        BINARY,
        &[],
        SWC_REST,
    ),
    variant(
        Kind::ObjectWithoutPropertiesLoose,
        0x1b27_c5ea_ac28_c1a9,
        (2, 7),
        IF,
        BINARY,
        &[],
        SWC_REST_MIN,
    ),
    variant(
        Kind::Rest,
        0xc3a5_613a_7fbd_7035,
        (2, 4),
        DECLARATION,
        OBJECT,
        &[],
        TS_REST,
    ),
    variant(
        Kind::Rest,
        0x3415_59b9_cbec_981f,
        (2, 4),
        DECLARATION,
        OBJECT,
        &[],
        TS_REST_MIN,
    ),
    variant(
        Kind::ObjRest,
        0xd927_46d4_be9f_9781,
        (2, 4),
        DECLARATION,
        OBJECT,
        OBJ_REST_PARTS,
        ESBUILD_REST,
    ),
];
