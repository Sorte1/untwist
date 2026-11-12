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
    hash: u128,
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
        0xa799_0048_4115_8499_941e_1308_9066_1633,
        2,
        3,
        IF,
        focus(ExpressionKind::LogicalExpression),
        &[],
        BABEL,
    ),
    variant(
        Kind::ArrayLikeToArray,
        0x3033_0a38_40ba_b215_b7c1_c82c_8bfe_7757,
        2,
        3,
        IF,
        focus(ExpressionKind::LogicalExpression),
        &[],
        SWC,
    ),
    variant(
        Kind::ArrayWithoutHoles,
        0x0a98_e546_e3ce_a609_1753_92ea_48af_7abb,
        1,
        1,
        IF,
        focus(ExpressionKind::CallExpression),
        ARRAY_LIKE,
        BABEL,
    ),
    variant(
        Kind::IterableToArray,
        0x0c4e_5568_3506_88e4_75cb_4841_c65d_e487,
        1,
        1,
        IF,
        focus(ExpressionKind::LogicalExpression),
        &[],
        BABEL,
    ),
    variant(
        Kind::IterableToArray,
        0x7271_8fc5_2371_25a6_18c7_f084_1c5a_1a02,
        1,
        1,
        IF,
        focus(ExpressionKind::LogicalExpression),
        &[],
        BABEL_MIN,
    ),
    variant(
        Kind::NonIterableSpread,
        0xdbff_9260_df3b_f36f_ea59_7aa1_743d_67b4,
        0,
        1,
        THROW,
        focus(ExpressionKind::NewExpression),
        &[],
        BABEL,
    ),
    variant(
        Kind::NonIterableSpread,
        0x866f_16a2_8e40_3fb8_d305_203a_404a_d85d,
        0,
        1,
        THROW,
        focus(ExpressionKind::NewExpression),
        &[],
        BABEL_MIN,
    ),
    variant(
        Kind::UnsupportedIterableToArray,
        0xaf67_776b_26b6_252d_8383_6f77_aa53_f510,
        2,
        1,
        IF,
        focus(ExpressionKind::Identifier),
        ARRAY_LIKE,
        BABEL,
    ),
    variant(
        Kind::UnsupportedIterableToArray,
        0x3756_9c03_18a4_b1b8_8535_7fff_655e_91cb,
        2,
        6,
        IF,
        focus(ExpressionKind::UnaryExpression),
        ARRAY_LIKE,
        SWC,
    ),
    variant(
        Kind::UnsupportedIterableToArray,
        0x1e1d_3836_561f_370e_aaf3_ba23_d24d_8774,
        2,
        1,
        IF,
        focus(ExpressionKind::Identifier),
        ARRAY_LIKE,
        SWC_MIN,
    ),
    variant(
        Kind::ToConsumableArray,
        0xfe67_2f08_01c1_5968_5dae_0814_1a34_c8cc,
        1,
        1,
        RETURN,
        focus(ExpressionKind::LogicalExpression),
        TO_CONSUMABLE_ARRAY_PARTS,
        BABEL,
    ),
    variant(
        Kind::ToConsumableArray,
        0xc1c5_270e_65e7_f93b_a05c_de94_a6e8_5f75,
        1,
        1,
        RETURN,
        focus(ExpressionKind::LogicalExpression),
        TO_CONSUMABLE_ARRAY_INLINED_PARTS,
        TERSER,
    ),
    variant(
        Kind::SpreadArray,
        0xe9e9_2b3c_7fd6_69de_649e_c621_c048_ec88,
        3,
        2,
        IF,
        focus(ExpressionKind::LogicalExpression),
        &[],
        TS,
    ),
    variant(
        Kind::SpreadArray,
        0x1c6f_ca4c_049e_e5bb_3133_82d9_5c1d_6b99,
        3,
        2,
        IF,
        focus(ExpressionKind::LogicalExpression),
        &[],
        TS_MIN,
    ),
    variant(
        Kind::Read,
        0x25ae_16d9_cc36_dd73_f6b2_0a59_5e3a_6e16,
        2,
        8,
        DECLARATION,
        focus(ExpressionKind::LogicalExpression),
        &[],
        TS_ITER,
    ),
    variant(
        Kind::Read,
        0x1ac5_3863_354b_3960_6878_e889_6619_40f8,
        2,
        8,
        DECLARATION,
        focus(ExpressionKind::LogicalExpression),
        &[],
        TS_ITER_MIN,
    ),
];
