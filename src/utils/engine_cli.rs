use std::collections::HashMap;
use crate::interface::min_max_eval;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::task::JoinSet;

pub fn cli_reader()
{
	let args: Vec<String> = std::env::args().collect();
	let mut hp: HashMap<String, String> = HashMap::new();
	for idx in 0..args.len()
	{
		//if start with -- then process else dont process
		if args[idx].starts_with("--")
		{
			let value = args.get(idx + 1).filter(|v| !v.starts_with("--")).cloned().unwrap_or_default();
			hp.insert(args[idx].clone(), value);
		}
	}

	// feature will be triggered on the basis of the environemnt variable at runtime provided
	//to populate the csv
	if hp.contains_key("--path_csv")
	{
		evaluate_csv(&hp);
	}
	else if  hp.contains_key("--eval_fen")
	{
		let depth: usize = hp.get("--depth").map(|s| s.as_str()).unwrap_or("3").parse().expect("--depth must be a whole number");
		let fen: &str = match hp.get("--eval_fen")
		{
			Some(s) => s.as_str(),
			None => panic!("pass the fen notation"),
		};
		let (score, best_move) = min_max_eval(fen, depth);
		println!("score: {score}");
		println!("best move: {best_move}");
	}
	//to get the min max score of the fen notation
}

const FEN: usize = 1;
const DEPTH: usize = 3;
const LICHESS_MOVE: usize = 4;
const LICHESS_VALUE: usize = 5;
const MOVE: usize = 6;
const VALUE: usize = 7;
const EVALUATED: usize = 8;

pub fn evaluate_csv(hp: &HashMap<String, String>)
{
	let path = hp.get("--path_csv").expect("pass the csv file path as --path_csv");
	tokio::runtime::Builder::new_current_thread()
		.enable_all()
		.build()
		.expect("cannot start the tokio runtime")
		.block_on(run_csv(path, hp.contains_key("--lichess"), hp.contains_key("--stockfish"), hp.contains_key("--engine_eval")));
}

async fn run_csv(path: &str, use_lichess: bool, use_stockfish: bool, use_engine: bool)
{
	let text = tokio::fs::read_to_string(path).await.expect("cannot open the file ,provide the valid path ");
	let mut lines = text.lines();
	let header = lines.next().expect("the csv file is empty");
	let mut rows: Vec<Vec<String>> = lines.map(|line| line.split(',').map(String::from).collect()).collect();

	let exe = std::env::current_exe().expect("cannot find the engine path");
	let engine_lock = Arc::new(Mutex::new(()));
	let lichess_lock = Arc::new(Mutex::new(()));
	let stockfish_lock = Arc::new(Mutex::new(()));

	let mut tasks = JoinSet::new();
	for (idx, row) in rows.iter().enumerate()
	{
		if row.len() <= EVALUATED || row[EVALUATED] == "true"
		{
			continue;
		}
		let fen = row[FEN].clone();
		let depth = row[DEPTH].clone();
		let exe = exe.clone();
		let engine_lock = engine_lock.clone();
		let lichess_lock = lichess_lock.clone();
		let stockfish_lock = stockfish_lock.clone();

		tasks.spawn(async move {
			let lichess = async {
				if use_lichess
				{
					let _guard = lichess_lock.lock().await;
					if let Some(result) = lichess_eval(&fen).await { return Some(result); }
				}
				if use_stockfish
				{
					let _guard = stockfish_lock.lock().await;
					return stockfish_eval(&fen).await;
				}
				None
			};
			let engine = async {
				if !use_engine { return None; }
				let _guard = engine_lock.lock().await;
				engine_eval(&exe, &fen, &depth).await
			};
			let (lichess, engine) = tokio::join!(lichess, engine);
			(idx, lichess, engine)
		});
	}

	while let Some(result) = tasks.join_next().await
	{
		let Ok((idx, lichess, engine)) = result else { continue };
		let row = &mut rows[idx];
		if use_lichess || use_stockfish
		{
			(row[LICHESS_MOVE], row[LICHESS_VALUE]) = lichess.unwrap_or_default();
		}
		if let Some(result) = engine
		{
			(row[MOVE], row[VALUE]) = result;
			row[EVALUATED] = "true".to_string();
		}
		println!("{}", row.join(","));
		tokio::fs::write(path, to_csv(header, &rows)).await.expect("cannot write the csv");
	}
}

fn to_csv(header: &str, rows: &[Vec<String>]) -> String
{
	let mut out = format!("{header}\n");
	for row in rows
	{
		out.push_str(&row.join(","));
		out.push('\n');
	}
	out
}

async fn engine_eval(exe: &Path, fen: &str, depth: &str) -> Option<(String, String)>
{
	let output = Command::new(exe).args(["--eval_fen", fen, "--depth", depth]).output().await.ok()?;
	let stdout = String::from_utf8(output.stdout).ok()?;
	let score = stdout.lines().find_map(|line| line.strip_prefix("score: "))?;
	let best_move = stdout.lines().find_map(|line| line.strip_prefix("best move: "))?;
	Some((best_move.to_string(), score.to_string()))
}

async fn lichess_eval(fen: &str) -> Option<(String, String)>
{
	for _ in 0..3
	{
		let output = Command::new("curl")
			.args(["-sG", "--max-time", "10", "-w", "\n%{http_code}",
				"https://lichess.org/api/cloud-eval", "--data-urlencode", &format!("fen={fen}")])
			.output()
			.await
			.ok()?;
		tokio::time::sleep(Duration::from_secs(1)).await;

		let text = String::from_utf8_lossy(&output.stdout);
		let (body, code) = text.rsplit_once('\n').unwrap_or(("", ""));
		match code
		{
			"200" => return parse_lichess(body),
			"404" => return None,
			"429" =>
			{
				eprintln!("lichess rate limited (429), waiting 60s");
				tokio::time::sleep(Duration::from_secs(60)).await;
			}
			_ => {}
		}
	}
	None
}

fn parse_lichess(body: &str) -> Option<(String, String)>
{
	let json: serde_json::Value = serde_json::from_str(body).ok()?;
	let pv = &json["pvs"][0];
	let best_move = pv["moves"].as_str()?.split_whitespace().next()?;
	let score = match pv["cp"].as_i64()
	{
		Some(cp) => cp.to_string(),
		None => format!("#{}", pv["mate"].as_i64()?),
	};
	Some((best_move.to_string(), score))
}

const STOCKFISH_DEPTH: u32 = 18;

async fn stockfish_eval(fen: &str) -> Option<(String, String)>
{
	let mut child = Command::new("stockfish")
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.kill_on_drop(true)
		.spawn()
		.ok()?;
	let mut stdin = child.stdin.take()?;
	stdin.write_all(format!("position fen {fen}\ngo depth {STOCKFISH_DEPTH}\n").as_bytes()).await.ok()?;

	let black_to_move = fen.split_whitespace().nth(1) == Some("b");
	let mut lines = BufReader::new(child.stdout.take()?).lines();
	let mut score = None;
	while let Some(line) = lines.next_line().await.ok()?
	{
		let words: Vec<&str> = line.split_whitespace().collect();
		if let Some(i) = words.iter().position(|w| *w == "score")
		{
			if let (Some(kind), Some(Ok(value))) = (words.get(i + 1), words.get(i + 2).map(|v| v.parse::<i64>()))
			{
				let value = if black_to_move { -value } else { value };
				score = Some(if *kind == "mate" { format!("#{value}") } else { value.to_string() });
			}
		}
		if words.first() == Some(&"bestmove")
		{
			stdin.write_all(b"quit\n").await.ok();
			return Some((words.get(1)?.to_string(), score?));
		}
	}
	None
}
