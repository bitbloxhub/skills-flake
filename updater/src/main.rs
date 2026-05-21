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
		#[arg(long = "repo", value_name = "REPO_OR_URL")]
		repo: Vec<String>,
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
			repo,
		} => {
			let skill_list_src =
				fs::read_to_string(&skill_list_kdl).map_err(|source| UpdaterError::ReadFile {
					path: skill_list_kdl.display().to_string(),
					source,
				})?;
			let mut parsed_skill_list_kdl = parse_skill_list_kdl(&skill_list_src)?;
			if !repo.is_empty() {
				let filters = normalize_repo_filters(&repo);
				parsed_skill_list_kdl.skills.retain(|source| {
					filters
						.iter()
						.any(|filter| source_matches_repo_filter(source, filter))
				});
				if parsed_skill_list_kdl.skills.is_empty() {
					return Err(UpdaterError::NoMatchingRepos {
						filters: filters.join(", "),
					});
				}
			}
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

fn normalize_repo_filters(repo_filters: &[String]) -> Vec<String> {
	let mut filters = repo_filters
		.iter()
		.flat_map(|raw| raw.split(','))
		.map(str::trim)
		.filter(|part| !part.is_empty())
		.map(ToString::to_string)
		.collect::<Vec<_>>();
	filters.sort();
	filters.dedup();
	filters
}

fn source_matches_repo_filter(source: &skills_flake_updater::SkillSource, filter: &str) -> bool {
	let (provider_filter, repo_filter_raw) = filter
		.split_once(':')
		.filter(|(left, _)| !left.contains("//"))
		.map_or((None, filter), |(provider, rest)| (Some(provider), rest));

	if provider_filter.is_some_and(|provider| provider != source.source) {
		return false;
	}

	let repo_filter = normalize_repo_like(repo_filter_raw);
	if normalize_repo_like(&source.repo) == repo_filter
		|| normalize_repo_like(&source.url) == repo_filter
	{
		return true;
	}

	if let Some((_, rest)) = source.url.split_once("://") {
		if let Some(path_start) = rest.find('/') {
			let path = &rest[path_start + 1..];
			if normalize_repo_like(path) == repo_filter {
				return true;
			}
		}
	}

	false
}

fn normalize_repo_like(value: &str) -> String {
	value
		.trim()
		.trim_end_matches(".git")
		.trim_end_matches('/')
		.to_string()
}
