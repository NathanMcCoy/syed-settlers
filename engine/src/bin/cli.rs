// The engine's command-line tool, for this PC (run from engine/ after cargo build --release):
//   target/release/cli island [--seed S] [--set "name=value;..."]   one island, as JSON
//   target/release/cli bench-island [--count N] [--set ...]        how fast islands are made
//   target/release/cli settings                                     every setting, as JSON
//   target/release/cli sim [--games N] [--players P] [--max-turns T] [--bots greedy,random] [--set ...]
//                                                                    bots play each other (--bots: by seat, repeated)
// Bot training will be added here.
use settlers::island::generate;
use settlers::settings::Settings;
use std::time::Instant;

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // --players also picks that many players' defaults (5-6: the large island)
    let players: usize = flag(&args, "--players").and_then(|c| c.parse().ok()).unwrap_or(4);
    let settings = match Settings::parse_for(&flag(&args, "--set").unwrap_or_default(), players) {
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
        Some("sim") => sim(&args, &settings),
        _ => fail("usage: cli island|bench-island|settings [--seed S] [--count N] [--set \"name=value;...\"]"),
    }
}

// Greedy bots play each other: how long games last, whether seat order matters, where Renown comes
// from, and how fast it all runs. Games are spread over every CPU thread.
fn sim(args: &[String], settings: &Settings) {
    use settlers::game::NOBODY;
    use settlers::sim::{BotKind, SOURCES, play_with};
    let kinds: Vec<BotKind> = flag(args, "--bots").unwrap_or_else(|| "greedy".into()).split(',')
        .map(|k| BotKind::parse(k.trim()).unwrap_or_else(|e| fail(&e))).collect();
    let games: u32 = flag(args, "--games").and_then(|c| c.parse().ok()).unwrap_or(1000);
    let players: usize = flag(args, "--players").and_then(|c| c.parse().ok()).unwrap_or(4);
    let max_turns: u32 = flag(args, "--max-turns").and_then(|c| c.parse().ok()).unwrap_or(400);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(games as usize).max(1);
    let started = Instant::now();
    let outcomes: Vec<_> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|t| { let kinds = &kinds; scope.spawn(move || {
                (t as u32..games).step_by(threads).map(|seed| play_with(settings, seed, players, max_turns, &kinds)).collect::<Vec<_>>()
            }) })
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    let secs = started.elapsed().as_secs_f64();
    let outcomes: Vec<_> = outcomes.into_iter().map(|o| o.unwrap_or_else(|e| fail(&e))).collect();
    let n = outcomes.len() as f64;
    let finished: Vec<_> = outcomes.iter().filter(|o| o.winner != NOBODY).collect();
    let actions: u64 = outcomes.iter().map(|o| o.actions as u64).sum();
    println!("{games} games, {players} players, on {threads} threads: {secs:.2} s ({:.0} games/s, {:.1} M actions/s)",
        n / secs, actions as f64 / secs / 1e6);
    println!("finished: {} ({:.1}%); the rest hit {max_turns} turns", finished.len(), 100.0 * finished.len() as f64 / n);
    if finished.is_empty() {
        return;
    }
    let mut turns: Vec<u32> = finished.iter().map(|o| o.turns).collect();
    turns.sort();
    let avg = turns.iter().sum::<u32>() as f64 / turns.len() as f64;
    println!("turns to win: average {avg:.1} ({:.1} rounds), median {}, 10%-90% {}-{}",
        avg / players as f64, turns[turns.len() / 2], turns[turns.len() / 10], turns[turns.len() * 9 / 10]);
    let wins: Vec<String> = (0..players)
        .map(|p| format!("seat {} {:.1}%", p + 1, 100.0 * finished.iter().filter(|o| o.winner as usize == p).count() as f64 / finished.len() as f64))
        .collect();
    let fair = 1.0 / players as f64;
    let margin = 100.0 * (fair * (1.0 - fair) / finished.len() as f64).sqrt() * 2.0; // 95%
    println!("wins by seat: {} (fair: {:.1}% ± {margin:.1})", wins.join(", "), 100.0 / players as f64);
    let mut sums = [0u32; 5];
    for o in &finished {
        for (k, v) in o.sources[o.winner as usize].iter().enumerate() {
            sums[k] += v;
        }
    }
    let total: u32 = sums.iter().sum();
    let parts: Vec<String> = SOURCES.iter().zip(sums).map(|(name, v)| format!("{name} {:.0}%", 100.0 * v as f64 / total as f64)).collect();
    println!("winners' Renown from: {}", parts.join(", "));
    // Tokens (e.g. the sea expansion's island bonus), when any were won
    let tokens: u32 = finished.iter().map(|o| o.token_renown[o.winner as usize]).sum();
    if tokens > 0 {
        println!("winners' Renown from tokens: {:.1} a game ({:.0}% of their Renown)",
            tokens as f64 / finished.len() as f64, 100.0 * tokens as f64 / (total + tokens) as f64);
    }
}

fn fail(why: &str) -> ! {
    eprintln!("{why}");
    std::process::exit(1);
}
