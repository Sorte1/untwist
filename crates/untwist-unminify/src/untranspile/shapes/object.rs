use untwist_core::pass::{ExpressionKind, StatementKind};

use super::{Alias, Kind, Variant};

const BABEL: &str = "fixtures/untranspile/babel/spread.7.29.js";
const SWC: &str = "fixtures/untranspile/swc/spread.es5.js";
const SWC_MIN: &str = "fixtures/untranspile/swc/spread.es5.min.js";
const TERSER: &str = "fixtures/untranspile/terser/spread.safari9.js";
const TS: &str = "fixtures/untranspile/ts/spread.es5.js";
const ESBUILD: &str = "fixtures/untranspile/esbuild/spread.0.28.1-es2015.js";
const ESBUILD_MIN: &str = "fixtures/untranspile/esbuild/spread.0.28.1-es2015.min.js";

const IF: u8 = StatementKind::IfStatement as u8;
const DECLARATION: u8 = StatementKind::VariableDeclaration as u8;
const EXPRESSION: u8 = StatementKind::ExpressionStatement as u8;
const FOR: u8 = StatementKind::ForStatement as u8;
const FOR_IN: u8 = StatementKind::ForInStatement as u8;
const EXPRESSION_BODY: u8 = u8::MAX;
const DECLARED_FOCUS: u8 = u8::MAX - 1;

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

const OBJECT_SPREAD2_PARTS: &[Kind] = &[Kind::OwnKeys, Kind::DefineProperty];
const TO_PROPERTY_KEY_PARTS: &[Kind] = &[Kind::ToPrimitive, Kind::Typeof];
const DEF_NORMAL_PROP_PARTS: &[Kind] = &[Kind::Alias(Alias::DefineProperty)];
const SPREAD_VALUES_PARTS: &[Kind] = &[
    Kind::Alias(Alias::HasOwnProperty),
    Kind::DefNormalProp,
    Kind::Alias(Alias::GetOwnPropertySymbols),
    Kind::Alias(Alias::PropertyIsEnumerable),
];
const SPREAD_PROPS_PARTS: &[Kind] = &[
    Kind::Alias(Alias::DefineProperties),
    Kind::Alias(Alias::GetOwnPropertyDescriptors),
];

pub const VARIANTS: &[Variant] = &[
    variant(
        Kind::Assign,
        0xa799_8b42_e2ae_a458_acc1_df6c_4419_e678,
        0,
        2,
        EXPRESSION,
        focus(ExpressionKind::AssignmentExpression),
        &[],
        TS,
    ),
    variant(
        Kind::ObjectSpread2,
        0x2828_7ae0_4812_c10e_735b_348d_6b58_20ba,
        1,
        2,
        FOR,
        DECLARED_FOCUS,
        OBJECT_SPREAD2_PARTS,
        BABEL,
    ),
    variant(
        Kind::OwnKeys,
        0x7a04_8884_cd1b_00c7_8150_cea6_88f3_d721,
        2,
        3,
        DECLARATION,
        focus(ExpressionKind::CallExpression),
        &[],
        BABEL,
    ),
    variant(
        Kind::DefineProperty,
        0xced7_6bd2_5ea5_a410_ea0e_fd15_32d6_830b,
        3,
        2,
        IF,
        focus(ExpressionKind::BinaryExpression),
        &[Kind::ToPropertyKey],
        BABEL,
    ),
    variant(
        Kind::ToPropertyKey,
        0xfa08_31c9_8644_27dc_694d_b7e3_e627_e931,
        1,
        2,
        DECLARATION,
        focus(ExpressionKind::CallExpression),
        TO_PROPERTY_KEY_PARTS,
        BABEL,
    ),
    variant(
        Kind::ToPropertyKey,
        0xc134_0fa3_27fd_e265_4276_a48b_385b_870a,
        1,
        2,
        DECLARATION,
        focus(ExpressionKind::CallExpression),
        &[],
        TERSER,
    ),
    variant(
        Kind::ToPrimitive,
        0x0f34_bce9_9c39_80cf_74f0_b034_fd22_8d21,
        2,
        4,
        IF,
        focus(ExpressionKind::LogicalExpression),
        &[Kind::Typeof],
        BABEL,
    ),
    variant(
        Kind::Typeof,
        0x64f0_08c3_a26d_5552_0371_410d_2f59_aba0,
        1,
        2,
        EXPRESSION,
        focus(ExpressionKind::AssignmentExpression),
        &[],
        BABEL,
    ),
    variant(
        Kind::DefinePropertyPlain,
        0x0da5_3186_d3a3_b53c_2995_d3dc_369a_1389,
        3,
        2,
        IF,
        focus(ExpressionKind::BinaryExpression),
        &[],
        SWC,
    ),
    variant(
        Kind::ObjectSpreadEach,
        0x2c9e_3826_d8b8_2264_ce26_bbfb_917d_ba6a,
        1,
        2,
        FOR,
        DECLARED_FOCUS,
        &[Kind::DefinePropertyPlain],
        SWC,
    ),
    variant(
        Kind::ObjectSpreadEach,
        0x92dd_5f0d_b10e_829d_7c29_a02e_64c8_e332,
        1,
        2,
        FOR,
        DECLARED_FOCUS,
        &[Kind::DefinePropertyPlain],
        SWC_MIN,
    ),
    variant(
        Kind::ObjectSpreadProps,
        0x3f31_b56e_7cc8_14a1_e35d_dc95_ceb2_2453,
        2,
        3,
        EXPRESSION,
        focus(ExpressionKind::AssignmentExpression),
        &[Kind::OwnKeys],
        SWC,
    ),
    variant(
        Kind::DefNormalProp,
        0x0e8e_6e0b_4c8b_07d2_594f_46ea_87b2_a16a,
        3,
        1,
        EXPRESSION_BODY,
        focus(ExpressionKind::ConditionalExpression),
        DEF_NORMAL_PROP_PARTS,
        ESBUILD,
    ),
    variant(
        Kind::DefNormalProp,
        0x34e6_0150_5eaa_9692_2837_07ca_fb4d_509d,
        3,
        1,
        EXPRESSION_BODY,
        focus(ExpressionKind::ConditionalExpression),
        DEF_NORMAL_PROP_PARTS,
        ESBUILD_MIN,
    ),
    variant(
        Kind::SpreadValues,
        0xe5e0_56cf_44f5_5355_a734_6f86_0635_30e9,
        2,
        3,
        FOR_IN,
        focus(ExpressionKind::AssignmentExpression),
        SPREAD_VALUES_PARTS,
        ESBUILD,
    ),
    variant(
        Kind::SpreadProps,
        0xe160_ce00_ce86_8993_3f77_4b25_548a_ee4c,
        2,
        1,
        EXPRESSION_BODY,
        focus(ExpressionKind::CallExpression),
        SPREAD_PROPS_PARTS,
        ESBUILD,
    ),
];
