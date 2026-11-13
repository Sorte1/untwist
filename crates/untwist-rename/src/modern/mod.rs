pub mod baseline;
pub mod collect;
pub mod compat;
pub mod edit;
pub mod lookalike;
pub mod pass;
pub mod plan;
pub mod regions;
pub mod report;
pub mod rewrite;
pub mod rewrite_apply;
pub mod trivial;

pub use baseline::Baseline;

pub fn parse_baseline(text: &str) -> Result<Baseline, String> {
    Baseline::parse(text, compat::builtin())
}
