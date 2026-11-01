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
        0xc9a8_5233_66c0_78d9,
        0,
        2,
        EXPRESSION,
        focus(ExpressionKind::AssignmentExpression),
        &[],
        TS,
    ),
    variant(
        Kind::ObjectSpread2,
        0xcad8_3b43_51cf_3045,
        1,
        2,
        FOR,
        DECLARED_FOCUS,
        OBJECT_SPREAD2_PARTS,
        BABEL,
    ),
    variant(
        Kind::OwnKeys,
        0x91b4_25b2_8032_11d0,
        2,
        3,
        DECLARATION,
        focus(ExpressionKind::CallExpression),
        &[],
        BABEL,
    ),
    variant(
        Kind::DefineProperty,
        0xb984_9799_8a37_8882,
        3,
        2,
        IF,
        focus(ExpressionKind::BinaryExpression),
        &[Kind::ToPropertyKey],
        BABEL,
    ),
    variant(
        Kind::ToPropertyKey,
        0xc317_41c6_bb50_d6d6,
        1,
        2,
        DECLARATION,
        focus(ExpressionKind::CallExpression),
        TO_PROPERTY_KEY_PARTS,
        BABEL,
    ),
    variant(
        Kind::ToPropertyKey,
        0x0a74_cafc_608b_6261,
        1,
        2,
        DECLARATION,
        focus(ExpressionKind::CallExpression),
        &[],
        TERSER,
    ),
    variant(
        Kind::ToPrimitive,
        0x875a_c24d_b883_b8dc,
        2,
        4,
        IF,
        focus(ExpressionKind::LogicalExpression),
        &[Kind::Typeof],
        BABEL,
    ),
    variant(
        Kind::Typeof,
        0x9a65_539d_29f1_6902,
        1,
        2,
        EXPRESSION,
        focus(ExpressionKind::AssignmentExpression),
        &[],
        BABEL,
    ),
    variant(
        Kind::DefinePropertyPlain,
        0x0e7f_1012_4efa_b4cd,
        3,
        2,
        IF,
        focus(ExpressionKind::BinaryExpression),
        &[],
        SWC,
    ),
    variant(
        Kind::ObjectSpreadEach,
        0x2bc5_a59e_6d34_836d,
        1,
        2,
        FOR,
        DECLARED_FOCUS,
        &[Kind::DefinePropertyPlain],
        SWC,
    ),
    variant(
        Kind::ObjectSpreadEach,
        0x24d7_4dd0_28c0_e3c7,
        1,
        2,
        FOR,
        DECLARED_FOCUS,
        &[Kind::DefinePropertyPlain],
        SWC_MIN,
    ),
    variant(
        Kind::ObjectSpreadProps,
        0x6463_b0a6_7a3f_d212,
        2,
        3,
        EXPRESSION,
        focus(ExpressionKind::AssignmentExpression),
        &[Kind::OwnKeys],
        SWC,
    ),
    variant(
        Kind::DefNormalProp,
        0xba43_68f3_3272_8869,
        3,
        1,
        EXPRESSION_BODY,
        focus(ExpressionKind::ConditionalExpression),
        DEF_NORMAL_PROP_PARTS,
        ESBUILD,
    ),
    variant(
        Kind::DefNormalProp,
        0x0b80_0a09_e18e_b69c,
        3,
        1,
        EXPRESSION_BODY,
        focus(ExpressionKind::ConditionalExpression),
        DEF_NORMAL_PROP_PARTS,
        ESBUILD_MIN,
    ),
    variant(
        Kind::SpreadValues,
        0xce04_c775_4067_49de,
        2,
        3,
        FOR_IN,
        focus(ExpressionKind::AssignmentExpression),
        SPREAD_VALUES_PARTS,
        ESBUILD,
    ),
    variant(
        Kind::SpreadProps,
        0x2b7d_65e1_1bd2_0d18,
        2,
        1,
        EXPRESSION_BODY,
        focus(ExpressionKind::CallExpression),
        SPREAD_PROPS_PARTS,
        ESBUILD,
    ),
];
