use std::{fs, io::IsTerminal, path::PathBuf};

use clap::{Parser, Subcommand};
use indicatif::ProgressStyle;
use skills_flake_updater::{
	Result as UpdaterResult, SkillsFlakeLock, UpdaterError, parse_skill_list_kdl,
	sort_skill_list_kdl, update,
};
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(version, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	Update {
		#[arg(long, default_value = "./skill-list.kdl")]
		skill_list_kdl: PathBuf,
		#[arg(long, default_value = "./skills-flake.lock.json")]
		skills_flake_lock_json: PathBuf,
	},
	SortSkillList {
		#[arg(long, default_value = "./skill-list.kdl")]
		skill_list_kdl: PathBuf,
	},
}
fn main() {
	if let Err(err) = smol::block_on(run()) {
		let mut out = String::new();
		if miette::GraphicalReportHandler::new()
			.render_report(&mut out, &err)
			.is_ok()
		{
			eprintln!("\n{out}");
		} else {
			eprintln!("{:#}", miette::Report::new(err));
		}
		std::process::exit(1);
	}
}

async fn run() -> UpdaterResult<()> {
	if use_dumb_logging() {
		tracing_subscriber::registry()
			.with(
				tracing_subscriber::fmt::layer()
					.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
					.with_writer(std::io::stderr),
			)
			.init();
	} else {
		let indicatif_layer = IndicatifLayer::new()
			.with_max_progress_bars(
				64,
				Some(
					ProgressStyle::with_template("… {pending_progress_bars} more hidden").unwrap(),
				),
			)
			.with_progress_style(
				ProgressStyle::with_template(
					"{span_child_prefix}{spinner} {span_name}{{{span_fields}}} {wide_msg}",
				)
				.unwrap(),
			);
		let stderr_writer = indicatif_layer.get_stderr_writer();

		tracing_subscriber::registry()
			.with(tracing_subscriber::fmt::layer().with_writer(stderr_writer))
			.with(indicatif_layer)
			.init();
	}

	match Cli::parse().command {
		Commands::Update {
			skill_list_kdl,
			skills_flake_lock_json,
		} => {
			let skill_list_src =
				fs::read_to_string(&skill_list_kdl).map_err(|source| UpdaterError::ReadFile {
					path: skill_list_kdl.display().to_string(),
					source,
				})?;
			let parsed_skill_list_kdl = parse_skill_list_kdl(&skill_list_src)?;
			let skills_flake_lock = if skills_flake_lock_json.exists() {
				let lock_src = fs::read_to_string(&skills_flake_lock_json).map_err(|source| {
					UpdaterError::ReadFile {
						path: skills_flake_lock_json.display().to_string(),
						source,
					}
				})?;
				serde_json::from_str::<SkillsFlakeLock>(&lock_src).map_err(|source| {
					UpdaterError::ParseJsonFile {
						path: skills_flake_lock_json.display().to_string(),
						source,
					}
				})?
			} else {
				SkillsFlakeLock::default()
			};
			let next_lock = update(parsed_skill_list_kdl, skills_flake_lock).await?;
			let json = serde_json::to_string_pretty(&next_lock)?;
			fs::write(&skills_flake_lock_json, format!("{}\n", json)).map_err(|source| {
				UpdaterError::WriteFile {
					path: skills_flake_lock_json.display().to_string(),
					source,
				}
			})?;
		}
		Commands::SortSkillList { skill_list_kdl } => {
			let skill_list_src =
				fs::read_to_string(&skill_list_kdl).map_err(|source| UpdaterError::ReadFile {
					path: skill_list_kdl.display().to_string(),
					source,
				})?;
			let sorted = sort_skill_list_kdl(&skill_list_src)?;
			fs::write(&skill_list_kdl, sorted).map_err(|source| UpdaterError::WriteFile {
				path: skill_list_kdl.display().to_string(),
				source,
			})?;
		}
	}

	Ok(())
}

fn use_dumb_logging() -> bool {
	let is_ci = std::env::var("CI")
		.map(|v| {
			let v = v.trim().to_ascii_lowercase();
			!v.is_empty() && v != "0" && v != "false"
		})
		.unwrap_or(false);
	std::env::var("TERM").map_or(false, |term| term == "dumb")
		|| !std::io::stderr().is_terminal()
		|| is_ci
}
