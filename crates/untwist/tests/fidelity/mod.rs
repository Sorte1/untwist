#![allow(dead_code, unused_macros)]

use crate::refused_rows::Row;

macro_rules! unit {
    ($decoder:literal) => {
        concat!(
            "function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }\n",
            $decoder,
            "\nfunction later(){ return b(1); }\nconsole.log(b(0)); console.log(later()); console.log(b(2));"
        )
    };
}

macro_rules! qunit {
    ($decoder:literal) => {
        concat!(
            "function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }\n",
            $decoder,
            "\nfunction later(){ return qb(1); }\nconsole.log(qb(0)); console.log(later()); console.log(qb(2));"
        )
    };
}

pub fn decoded(source: &'static str, prints: &'static str) -> Row {
    Row {
        source,
        prints,
        kept: "console.log(\"s0|",
    }
}

pub fn q_refused(source: &'static str, prints: &'static str) -> Row {
    Row {
        source,
        prints,
        kept: "console.log(qb(0))",
    }
}

pub fn agrees(source: &'static str, prints: &'static str) -> Row {
    Row {
        source,
        prints,
        kept: "console.log(",
    }
}

pub fn refused(source: &'static str, prints: &'static str) -> Row {
    Row {
        source,
        prints,
        kept: "console.log(b(0))",
    }
}
