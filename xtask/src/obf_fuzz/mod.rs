pub mod generate;
pub mod judge;
pub mod ledger;
pub mod minimise;
pub mod report;
pub mod spec;

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{bail, Context, Result};

use crate::obf_matrix::{pool, runner};
use crate::obf_options::lanes::UNTWIST_LANES;
use judge::{judge_sample, Rules, SampleResult, WRONG_OUTPUT};
use minimise::{minimise, Target};
use report::Cluster;
use spec::Spec;

#[derive(clap::Args)]
pub struct Request {
    #[arg(
        long,
        default_value = "1",
        help = "seed of the whole run; each sample is drawn from seed and index"
    )]
    pub seed: String,
    #[arg(long, default_value_t = 200)]
    pub count: usize,
    #[arg(long, default_value_t = 8)]
    pub jobs: usize,
    #[arg(long)]
    pub untwist: Option<PathBuf>,
    #[arg(long, help = "work directory (default target/obf-fuzz/seed-<seed>)")]
    pub out: Option<PathBuf>,
    #[arg(
        long,
        default_value = "new",
        value_parser = ["new", "all", "none"],
        help = "which failure clusters get a minimised repro"
    )]
    pub minimise: String,
}

fn judge_all(
    runner: &crate::obf_matrix::cell::Runner,
    dirs: Vec<PathBuf>,
    jobs: usize,
    rules: &Rules<'_>,
) -> Result<Vec<SampleResult>> {
    let results = pool(dirs, jobs, |dir| {
        let spec = Spec::load(&dir)?;
        let result = judge_sample(runner, &spec, &UNTWIST_LANES, rules);
        for lane in UNTWIST_LANES {
            let _ = std::fs::remove_dir_all(dir.join(lane));
        }
        result.with_context(|| dir.display().to_string())
    });
    results.into_iter().collect()
}

fn wanted(cluster: &Cluster, mode: &str) -> bool {
    match mode {
        "all" => cluster.tag != "SOUND",
        "new" => cluster.is_new(),
        _ => false,
    }
}

fn minimise_clusters(
    (root, runner, node): (&Path, &crate::obf_matrix::cell::Runner, &Path),
    rules: &Rules<'_>,
    (out, mode, jobs): (&Path, &str, usize),
    clusters: &mut [Cluster],
) -> Result<()> {
    let repro = out.join("repro");
    std::fs::create_dir_all(&repro)?;
    let picked: Vec<usize> = (0..clusters.len())
        .filter(|&i| wanted(&clusters[i], mode))
        .collect();
    let done = pool(picked, jobs, |index| {
        let cluster = &clusters[index];
        let spec = Spec::load(&out.join(&cluster.samples[0]))?;
        let target = Target {
            lane: cluster.lane,
            cause: &cluster.cause,
        };
        let found = minimise((root, runner, node), rules, &spec.raw, target, &repro)?;
        println!(
            "minimised {} {} from {} to {} chunk(s) in {} probe(s): {}",
            cluster.lane,
            cluster.cause,
            spec.raw["chunks"].as_array().map_or(0, Vec::len),
            found.chunks,
            found.probes,
            found.dir.display()
        );
        Ok::<_, anyhow::Error>((index, found.dir))
    });
    for item in done {
        match item {
            Ok((index, dir)) => clusters[index].repro = Some(dir),
            Err(error) => println!("minimise: {error:#}"),
        }
    }
    Ok(())
}

pub fn run(root: &Path, request: &Request) -> Result<()> {
    let started = Instant::now();
    let runner = runner(root, request.untwist.as_ref(), true)?;
    let Some(node) = runner.node.clone() else {
        bail!("obf-fuzz needs node: it generates the samples and runs the original and the output");
    };
    let refusals = ledger::load_refusals(root)?;
    let known = ledger::load_known(root)?;
    let rules = Rules {
        refusals: &refusals,
        known: &known,
    };
    let out = request.out.clone().unwrap_or_else(|| {
        root.join("target/obf-fuzz")
            .join(format!("seed-{}", request.seed))
    });
    let dirs = generate::generate(
        root,
        &node,
        (&request.seed, request.count),
        request.jobs,
        &out,
    )?;
    let generated = started.elapsed().as_secs_f64();
    let results = judge_all(&runner, dirs, request.jobs, &rules)?;
    let mut clusters = report::clusters(&results);
    minimise_clusters(
        (root, &runner, &node),
        &rules,
        (&out, &request.minimise, request.jobs),
        &mut clusters,
    )?;
    let text = report::render(&clusters, &report::totals(&results));
    print!("{text}");
    report::write(&out.join("report.tsv"), &text)?;
    report::write(&out.join("samples.tsv"), &report::render_samples(&results))?;
    for cluster in clusters.iter().filter(|c| c.cause == WRONG_OUTPUT) {
        println!(
            "WRONG OUTPUT: {} lane, {} sample(s), first {}, repro {}",
            cluster.lane,
            cluster.samples.len(),
            out.join(&cluster.samples[0]).display(),
            cluster
                .repro
                .as_ref()
                .map_or("-".to_string(), |p| p.display().to_string())
        );
    }
    println!(
        "obf-fuzz: seed {} count {}, generated in {generated:.1} s, wall {:.1} s, work {}",
        request.seed,
        request.count,
        started.elapsed().as_secs_f64(),
        out.display()
    );
    let new = report::new_failures(&clusters);
    if new > 0 {
        bail!(
            "{new} lane run(s) failed for a cause that is neither a sound refusal nor a known gap"
        );
    }
    Ok(())
}
