use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use untwist::{Options, Pipeline};

const FIXTURES: [&str; 3] = [
    "fixtures/reference/deobfuscate/obfuscator.io-high.js",
    "fixtures/reference/unpack/webpack-5-no-entry.js",
    "fixtures/reference/unpack/browserify-cocos2d.js",
];

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bench(c: &mut Criterion) {
    let options = Options {
        deobfuscate: false,
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    let pipeline = Pipeline::new(options);
    let mut group = c.benchmark_group("unminify");
    for fixture in FIXTURES {
        let path = root().join(fixture);
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(fixture), &source, |b, src| {
            b.iter(|| pipeline.run(src));
        });
    }
    group.finish();
}

const DEOBFUSCATE_FIXTURES: [&str; 3] = [
    "fixtures/reference/deobfuscate/obfuscator.io-high.js",
    "fixtures/reference/deobfuscate/obfuscator.io-multi-encoders.js",
    "fixtures/reference/deobfuscate/obfuscator.io.js",
];

fn bench_deobfuscate(c: &mut Criterion) {
    let options = Options {
        unpack: false,
        jsx: false,
        unsafe_transforms: true,
        ..Options::default()
    };
    let pipeline = Pipeline::new(options);
    let mut group = c.benchmark_group("deobfuscate");
    for fixture in DEOBFUSCATE_FIXTURES {
        let path = root().join(fixture);
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(fixture), &source, |b, src| {
            b.iter(|| pipeline.run(src));
        });
    }
    group.finish();
}

criterion_group!(benches, bench, bench_deobfuscate);
criterion_main!(benches);
