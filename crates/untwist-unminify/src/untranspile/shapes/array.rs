use untwist_core::pass::{ExpressionKind, StatementKind};

use super::{Kind, Variant};

const BABEL: &str = "fixtures/untranspile/babel/spread.7.29.js";
const BABEL_MIN: &str = "fixtures/untranspile/babel/spread.7.29.min.js";
const SWC: &str = "fixtures/untranspile/swc/spread.es5.js";
const SWC_MIN: &str = "fixtures/untranspile/swc/spread.es5.min.js";
const TERSER: &str = "fixtures/untranspile/terser/spread.safari9.js";
const TS: &str = "fixtures/untranspile/ts/spread.es5.js";
const TS_MIN: &str = "fixtures/untranspile/ts/spread.es5.min.js";
const TS_ITER: &str = "fixtures/untranspile/ts/spread.es5-iter.js";
const TS_ITER_MIN: &str = "fixtures/untranspile/ts/spread.es5-iter.min.js";

const IF: u8 = StatementKind::IfStatement as u8;
const RETURN: u8 = StatementKind::ReturnStatement as u8;
const THROW: u8 = StatementKind::ThrowStatement as u8;
const DECLARATION: u8 = StatementKind::VariableDeclaration as u8;

const fn focus(kind: ExpressionKind) -> u8 {
    kind as u8
}

#[allow(clippy::too_many_arguments)]
const fn variant(
    kind: Kind,
    hash: u64,
    params: u8,
    statements: u8,
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

const TO_CONSUMABLE_ARRAY_PARTS: &[Kind] = &[
    Kind::ArrayWithoutHoles,
    Kind::IterableToArray,
    Kind::UnsupportedIterableToArray,
    Kind::NonIterableSpread,
];

const TO_CONSUMABLE_ARRAY_INLINED_PARTS: &[Kind] = &[
    Kind::ArrayLikeToArray,
    Kind::IterableToArray,
    Kind::UnsupportedIterableToArray,
];

const ARRAY_LIKE: &[Kind] = &[Kind::ArrayLikeToArray];

pub const VARIANTS: &[Variant] = &[
    variant(
        Kind::ArrayLikeToArray,
        0xc3e9_dc77_27e3_2061,
        2,
        3,
        IF,
        focus(ExpressionKind::LogicalExpression),
        &[],
        BABEL,
    ),
    variant(
        Kind::ArrayLikeToArray,
        0xfc69_980f_bc8e_4f50,
        2,
        3,
        IF,
        focus(ExpressionKind::LogicalExpression),
        &[],
        SWC,
    ),
    variant(
        Kind::ArrayWithoutHoles,
        0x4172_a772_5914_ce57,
        1,
        1,
        IF,
        focus(ExpressionKind::CallExpression),
        ARRAY_LIKE,
        BABEL,
    ),
    variant(
        Kind::IterableToArray,
        0x3e96_7b56_1381_b752,
        1,
        1,
        IF,
        focus(ExpressionKind::LogicalExpression),
        &[],
        BABEL,
    ),
    variant(
        Kind::IterableToArray,
        0x38e1_5cf9_7f7d_14cc,
        1,
        1,
        IF,
        focus(ExpressionKind::LogicalExpression),
        &[],
        BABEL_MIN,
    ),
    variant(
        Kind::NonIterableSpread,
        0xcebd_f232_ed8c_c40c,
        0,
        1,
        THROW,
        focus(ExpressionKind::NewExpression),
        &[],
        BABEL,
    ),
    variant(
        Kind::NonIterableSpread,
        0x8f28_52a8_9c11_af19,
        0,
        1,
        THROW,
        focus(ExpressionKind::NewExpression),
        &[],
        BABEL_MIN,
    ),
    variant(
        Kind::UnsupportedIterableToArray,
        0xadbf_fac5_b0d7_f239,
        2,
        1,
        IF,
        focus(ExpressionKind::Identifier),
        ARRAY_LIKE,
        BABEL,
    ),
    variant(
        Kind::UnsupportedIterableToArray,
        0xe316_7027_e68b_f447,
        2,
        6,
        IF,
        focus(ExpressionKind::UnaryExpression),
        ARRAY_LIKE,
        SWC,
    ),
    variant(
        Kind::UnsupportedIterableToArray,
        0x2a24_cd8d_35db_e297,
        2,
        1,
        IF,
        focus(ExpressionKind::Identifier),
        ARRAY_LIKE,
        SWC_MIN,
    ),
    variant(
        Kind::ToConsumableArray,
        0x736b_03e3_e90c_3ca3,
        1,
        1,
        RETURN,
        focus(ExpressionKind::LogicalExpression),
        TO_CONSUMABLE_ARRAY_PARTS,
        BABEL,
    ),
    variant(
        Kind::ToConsumableArray,
        0x34e7_5e39_b341_b066,
        1,
        1,
        RETURN,
        focus(ExpressionKind::LogicalExpression),
        TO_CONSUMABLE_ARRAY_INLINED_PARTS,
        TERSER,
    ),
    variant(
        Kind::SpreadArray,
        0x7c6d_a228_7d45_e678,
        3,
        2,
        IF,
        focus(ExpressionKind::LogicalExpression),
        &[],
        TS,
    ),
    variant(
        Kind::SpreadArray,
        0x0860_148f_8727_9367,
        3,
        2,
        IF,
        focus(ExpressionKind::LogicalExpression),
        &[],
        TS_MIN,
    ),
    variant(
        Kind::Read,
        0x77ad_8f1a_d51f_b2fc,
        2,
        8,
        DECLARATION,
        focus(ExpressionKind::LogicalExpression),
        &[],
        TS_ITER,
    ),
    variant(
        Kind::Read,
        0xf72e_9e4b_1c7d_313e,
        2,
        8,
        DECLARATION,
        focus(ExpressionKind::LogicalExpression),
        &[],
        TS_ITER_MIN,
    ),
];
