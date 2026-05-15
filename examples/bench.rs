use std::env;
use std::error::Error;
use std::fs;
use std::hint::black_box;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use xgwx::XgwxDocument;

const DEFAULT_WARMUP: usize = 20;
const DEFAULT_ITERATIONS: usize = 200;

fn main() -> Result<(), Box<dyn Error>> {
    let config = BenchConfig::from_args(env::args().skip(1))?;
    let fixtures = load_fixtures(&config.paths)?;

    if fixtures.is_empty() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "no .xgwx fixtures found").into());
    }

    println!(
        "fixtures={} warmup={} iterations={}",
        fixtures.len(),
        config.warmup,
        config.iterations
    );
    println!(
        "{:<28} {:>9} {:>14} {:>14} {:>14} {:>14}",
        "fixture", "bytes", "parse avg", "parse min", "decode avg", "decode min"
    );

    for fixture in fixtures {
        warmup(&fixture.bytes, config.warmup)?;

        let parse = run_iterations(config.iterations, || {
            let doc = XgwxDocument::parse(&fixture.bytes)?;
            black_box(doc);
            Ok(())
        })?;

        let decode = run_iterations(config.iterations, || {
            let checksum = decode_all(&fixture.bytes)?;
            black_box(checksum);
            Ok(())
        })?;

        println!(
            "{:<28} {:>9} {:>14} {:>14} {:>14} {:>14}",
            fixture.name,
            fixture.bytes.len(),
            format_duration(parse.average()),
            format_duration(parse.min),
            format_duration(decode.average()),
            format_duration(decode.min)
        );
    }

    Ok(())
}

fn warmup(bytes: &[u8], iterations: usize) -> Result<(), Box<dyn Error>> {
    for _ in 0..iterations {
        black_box(decode_all(bytes)?);
    }
    Ok(())
}

fn run_iterations(
    iterations: usize,
    mut run: impl FnMut() -> Result<(), Box<dyn Error>>,
) -> Result<Measurement, Box<dyn Error>> {
    if iterations == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "iterations must be > 0").into());
    }

    let mut total = Duration::ZERO;
    let mut min = Duration::MAX;

    for _ in 0..iterations {
        let start = Instant::now();
        run()?;
        let elapsed = start.elapsed();
        total += elapsed;
        min = min.min(elapsed);
    }

    Ok(Measurement {
        total,
        min,
        iterations,
    })
}

fn decode_all(bytes: &[u8]) -> Result<usize, Box<dyn Error>> {
    let doc = XgwxDocument::parse(bytes)?;
    let project = doc.project_info();
    let variables = doc.variables()?;
    let ladder = doc.ladder_programs();
    let decoded_payloads = doc.decoded_payloads();
    let hsc_parameters = doc.hsc_parameters();

    let checksum = project.name.as_ref().map_or(0, String::len)
        + doc.configurations().len()
        + doc.networks().len()
        + doc.network_modules().len()
        + doc.bases().len()
        + doc.modules().len()
        + doc.tasks().len()
        + doc.programs().len()
        + variables.len()
        + ladder.iter().filter(|item| item.is_ok()).count()
        + decoded_payloads.iter().filter(|item| item.is_ok()).count()
        + hsc_parameters.iter().filter(|item| item.is_ok()).count()
        + doc.high_speed_links().len()
        + doc.high_speed_link_blocks().len()
        + doc.project_options().map_or(0, |_| 1)
        + doc.parameters().len()
        + doc.position_parameters().len()
        + doc.pid_cal_parameters().len()
        + doc.pid_tune_parameters().len()
        + doc.safety_comm().map_or(0, |_| 1)
        + doc.trend_monitoring().map_or(0, |_| 1)
        + doc.xgpd_config_infos().len()
        + doc.cnet_config_infos().len()
        + doc.fenet_config_infos().len()
        + doc.properties().len();

    Ok(checksum)
}

fn load_fixtures(paths: &[PathBuf]) -> Result<Vec<Fixture>, Box<dyn Error>> {
    let mut fixtures = Vec::new();

    for path in paths {
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                if is_xgwx(&path) {
                    fixtures.push(load_fixture(path)?);
                }
            }
        } else if is_xgwx(path) {
            fixtures.push(load_fixture(path.clone())?);
        }
    }

    fixtures.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(fixtures)
}

fn load_fixture(path: PathBuf) -> Result<Fixture, Box<dyn Error>> {
    let bytes = fs::read(&path)?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<unknown>")
        .to_owned();

    Ok(Fixture { name, bytes })
}

fn is_xgwx(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("xgwx"))
}

fn format_duration(duration: Duration) -> String {
    let nanos = duration.as_nanos();

    if nanos >= 1_000_000 {
        format!("{:.3} ms", nanos as f64 / 1_000_000.0)
    } else if nanos >= 1_000 {
        format!("{:.3} us", nanos as f64 / 1_000.0)
    } else {
        format!("{nanos} ns")
    }
}

#[derive(Debug)]
struct BenchConfig {
    paths: Vec<PathBuf>,
    warmup: usize,
    iterations: usize,
}

impl BenchConfig {
    fn from_args(args: impl IntoIterator<Item = String>) -> Result<Self, Box<dyn Error>> {
        let mut paths = Vec::new();
        let mut warmup = DEFAULT_WARMUP;
        let mut iterations = DEFAULT_ITERATIONS;
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                "--warmup" => {
                    let value = args.next().ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidInput, "--warmup needs a value")
                    })?;
                    warmup = value.parse()?;
                }
                "--iterations" | "-n" => {
                    let value = args.next().ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidInput, "--iterations needs a value")
                    })?;
                    iterations = value.parse()?;
                }
                path => paths.push(PathBuf::from(path)),
            }
        }

        if paths.is_empty() {
            paths.push(PathBuf::from("fixtures"));
        }

        Ok(Self {
            paths,
            warmup,
            iterations,
        })
    }
}

#[derive(Debug)]
struct Fixture {
    name: String,
    bytes: Vec<u8>,
}

#[derive(Debug)]
struct Measurement {
    total: Duration,
    min: Duration,
    iterations: usize,
}

impl Measurement {
    fn average(&self) -> Duration {
        self.total / self.iterations as u32
    }
}

fn print_usage() {
    eprintln!(
        "usage: cargo run --release --example bench -- [--warmup N] [--iterations N] [fixtures|file.xgwx ...]"
    );
}
