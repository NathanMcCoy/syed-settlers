// The engine's command-line tool, for this PC (run from engine/ after cargo build --release):
//   target/release/cli island [--seed S] [--set "name=value;..."]   one island, as JSON
//   target/release/cli bench-island [--count N] [--set ...]        how fast islands are made
//   target/release/cli settings                                     every setting, as JSON
// Simulations and bot training will be added here.
use settlers::island::generate;
use settlers::settings::Settings;
use std::time::Instant;

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let settings = match Settings::parse(&flag(&args, "--set").unwrap_or_default()) {
        Ok(s) => s,
        Err(e) => fail(&e),
    };
    match args.first().map(String::as_str) {
        Some("island") => {
            let seed = flag(&args, "--seed").unwrap_or_else(|| "default".into());
            match generate(&settings, &seed) {
                Ok(island) => println!("{}", island.to_json()),
                Err(e) => fail(&e),
            }
        }
        Some("bench-island") => {
            let count: usize = flag(&args, "--count").and_then(|c| c.parse().ok()).unwrap_or(10_000);
            let started = Instant::now();
            let mut attempts = 0u64;
            for i in 0..count {
                match generate(&settings, &format!("bench{i}")) {
                    Ok(island) => attempts += island.info.attempts as u64,
                    Err(e) => fail(&e),
                }
            }
            let s = started.elapsed().as_secs_f64();
            println!("{count} islands in {s:.2} s: {:.0} Âµs each, {:.2} attempts on average",
                s * 1e6 / count as f64, attempts as f64 / count as f64);
        }
        Some("settings") => println!("{}", Settings::table_json()),
        _ => fail("usage: cli island|bench-island|settings [--seed S] [--count N] [--set \"name=value;...\"]"),
    }
}

fn fail(why: &str) -> ! {
    eprintln!("{why}");
    std::process::exit(1);
}
