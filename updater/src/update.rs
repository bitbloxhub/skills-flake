use std::{collections::BTreeMap, io::IsTerminal};

use futures_util::stream::{FuturesUnordered, StreamExt};
use indicatif::ProgressStyle;
use owo_colors::OwoColorize;
use tracing::{info, info_span};
use tracing_indicatif::span_ext::IndicatifSpanExt;

use crate::{
	Result, UpdaterError,
	git::{fetch_skill_dirs, prefetch_skill},
	model::{LockNode, ParsedSkillListKdl, SkillLockEntry, SkillSource, SkillsFlakeLock},
};

fn locked_rev_for_source(
	root: &BTreeMap<String, LockNode>,
	source: &SkillSource,
) -> Option<String> {
	let mut node = root.get(&source.source)?;
	for part in source.repo.split('/').filter(|p| !p.is_empty()) {
		node = match node {
			LockNode::Branch(children) => children.get(part)?,
			LockNode::Entry(_) => return None,
		};
	}
	let mut rev: Option<String> = None;
	fn visit(node: &LockNode, rev: &mut Option<String>) -> Option<()> {
		match node {
			LockNode::Entry(entry) => {
				let entry_rev = entry.source.rev.as_ref()?.clone();
				match rev {
					Some(r) if r != &entry_rev => None,
					Some(_) => Some(()),
					None => {
						*rev = Some(entry_rev);
						Some(())
					}
				}
			}
			LockNode::Branch(children) => {
				for child in children.values() {
					visit(child, rev)?;
				}
				Some(())
			}
		}
	}
	visit(node, &mut rev)?;
	rev
}

fn lock_has_skill(root: &BTreeMap<String, LockNode>, source: &SkillSource, skill: &str) -> bool {
	let mut node = match root.get(&source.source) {
		Some(n) => n,
		None => return false,
	};
	for part in source.repo.split('/').filter(|p| !p.is_empty()) {
		node = match node {
			LockNode::Branch(children) => match children.get(part) {
				Some(n) => n,
				None => return false,
			},
			LockNode::Entry(_) => return false,
		};
	}
	for part in skill.split('/').filter(|p| !p.is_empty()) {
		node = match node {
			LockNode::Branch(children) => match children.get(part) {
				Some(n) => n,
				None => return false,
			},
			LockNode::Entry(_) => return false,
		};
	}
	matches!(node, LockNode::Entry(_))
}

#[derive(Default)]
struct RepoReport {
	found: u64,
	skipped: u64,
	prefetched: Vec<String>,
}

pub async fn update(
	parsed_skill_list_kdl: ParsedSkillListKdl,
	mut skills_flake_lock: SkillsFlakeLock,
) -> Result<SkillsFlakeLock> {
	skills_flake_lock.version = if skills_flake_lock.version == 0 {
		1
	} else {
		skills_flake_lock.version
	};

	let use_progress = use_progress_ui();
	let log_info = std::env::var("TERM").map_or(false, |term| term == "dumb");
	const MAX_INFLIGHT_PREFETCH: usize = 16;

	let mut total_tasks = 0_u64;
	let mut completed = 0_u64;
	let mut source_tasks = FuturesUnordered::new();
	let mut prefetch_tasks = FuturesUnordered::new();
	let mut pending_prefetch: Vec<(SkillSource, String, tracing::Span, String)> = Vec::new();
	let mut repo_report: BTreeMap<String, RepoReport> = BTreeMap::new();
	let header_span = info_span!("header");
	if use_progress {
		header_span.pb_set_style(&ProgressStyle::with_template("{wide_msg} {pos}/{len}").unwrap());
		header_span.pb_set_length(0);
		header_span.pb_set_message(&render_gradient_bar(
			completed,
			total_tasks,
			terminal_bar_width()?,
		));
	}
	let _header_enter = header_span.enter();

	for source in parsed_skill_list_kdl.skills {
		total_tasks += 1;
		if use_progress {
			header_span.pb_set_length(total_tasks);
			header_span.pb_set_message(&render_gradient_bar(
				completed,
				total_tasks,
				terminal_bar_width()?,
			));
		}
		let use_progress_for_scan = use_progress;
		let parent_span = header_span.clone();
		source_tasks.push(smol::spawn(async move {
			let scan_span = info_span!(parent: &parent_span, "scan.repo", url = %source.url);
			if use_progress_for_scan {
				let scan_style = ProgressStyle::with_template(
					"{span_child_prefix}{spinner:.cyan} {span_name}{{{span_fields}}} {msg}",
				)
				.unwrap();
				scan_span.pb_set_style(&scan_style);
				scan_span.pb_set_message("scanning");
				scan_span.pb_start();
			}
			let source_clone = source.clone();
			let (resolved_rev, skills) = {
				let _scan_guard = scan_span.enter();
				smol::unblock(move || fetch_skill_dirs(&source_clone)).await?
			};
			if use_progress_for_scan {
				scan_span.pb_set_message(&format!("scanned skills={}", skills.len()));
			}
			Ok::<_, UpdaterError>((source, resolved_rev, skills, scan_span))
		}));
	}

	while let Some(scan_result) = source_tasks.next().await {
		let (source, resolved_rev, skills, scan_span) = scan_result?;
		repo_report.entry(source.url.clone()).or_default().found = skills.len() as u64;
		let mut skills_to_prefetch = skills.clone();

		if locked_rev_for_source(&skills_flake_lock.source, &source).as_deref()
			== Some(&resolved_rev)
		{
			let total_before = skills_to_prefetch.len() as u64;
			skills_to_prefetch
				.retain(|skill| !lock_has_skill(&skills_flake_lock.source, &source, skill));
			let skipped_skills = total_before.saturating_sub(skills_to_prefetch.len() as u64);
			if skipped_skills > 0 {
				repo_report.entry(source.url.clone()).or_default().skipped += skipped_skills;
			}
			if skills_to_prefetch.is_empty() {
				completed += 1;
				if use_progress {
					header_span.pb_inc(1);
					header_span.pb_set_message(&render_gradient_bar(
						completed,
						total_tasks,
						terminal_bar_width()?,
					));
				}
				continue;
			}
		}

		total_tasks += skills_to_prefetch.len() as u64;
		if use_progress {
			header_span.pb_set_length(total_tasks);
			header_span.pb_set_message(&render_gradient_bar(
				completed,
				total_tasks,
				terminal_bar_width()?,
			));
		}
		if log_info {
			info!(url = %source.url, skills = skills.len(), "scan.repo");
		}
		completed += 1;
		if use_progress {
			header_span.pb_inc(1);
			header_span.pb_set_message(&render_gradient_bar(
				completed,
				total_tasks,
				terminal_bar_width()?,
			));
		} else if log_info {
			info!(url = %source.url, skills = skills.len(), "scan.repo.done");
		}
		for skill in skills_to_prefetch {
			let source_for_prefetch = SkillSource {
				rev: Some(resolved_rev.clone()),
				branch: None,
				tag: None,
				..source.clone()
			};
			pending_prefetch.push((
				source_for_prefetch,
				skill,
				scan_span.clone(),
				source.url.clone(),
			));
		}
	}

	for (source_for_prefetch, skill_for_prefetch, prefetch_parent, url_for_span) in pending_prefetch
	{
		while prefetch_tasks.len() >= MAX_INFLIGHT_PREFETCH {
			if let Some(prefetch_result) = prefetch_tasks.next().await {
				let (skill, lock_entry): (String, SkillLockEntry) = prefetch_result?;
				let key_path =
					hierarchy_path(&lock_entry.source.source, &lock_entry.source.repo, &skill);
				let lock_url = lock_entry.source.url.clone();
				insert_lock_entry(&mut skills_flake_lock.source, &key_path, lock_entry);
				repo_report
					.entry(lock_url)
					.or_default()
					.prefetched
					.push(skill.clone());
				completed += 1;
				if use_progress {
					header_span.pb_inc(1);
					header_span.pb_set_message(&render_gradient_bar(
						completed,
						total_tasks,
						terminal_bar_width()?,
					));
				} else if log_info {
					info!(skill = %skill, "prefetch.skill.done");
				}
			}
		}

		let source_for_prefetch_clone = source_for_prefetch.clone();
		let use_progress_for_prefetch = use_progress;
		let skill_span = info_span!(
			parent: &prefetch_parent,
			"prefetch.skill",
			url = %url_for_span,
			skill = %skill_for_prefetch
		);
		if use_progress_for_prefetch {
			let skill_style = ProgressStyle::with_template(
				"{span_child_prefix}{spinner:.cyan} {span_name}{{{span_fields}}} {msg}",
			)
			.unwrap();
			skill_span.pb_set_style(&skill_style);
			skill_span.pb_set_message("prefetching");
			skill_span.pb_start();
		}

		prefetch_tasks.push(smol::spawn(async move {
			if log_info {
				info!(url = %url_for_span, skill = %skill_for_prefetch, "prefetch.skill");
			}
			let _skill_guard = skill_span.enter();
			let skill_for_prefetch_unblock = skill_for_prefetch.clone();
			let prefetch = smol::unblock(move || {
				prefetch_skill(&source_for_prefetch_clone, &skill_for_prefetch_unblock)
			})
			.await?;
			let source_for_skill = SkillSource {
				rev: Some(prefetch.rev.clone()),
				..source_for_prefetch
			};
			let lock_entry = SkillLockEntry {
				hash: prefetch.hash,
				source: source_for_skill,
			};
			Ok::<_, UpdaterError>((skill_for_prefetch, lock_entry))
		}));
	}

	while let Some(prefetch_result) = prefetch_tasks.next().await {
		let (skill, lock_entry): (String, SkillLockEntry) = prefetch_result?;
		let key_path = hierarchy_path(&lock_entry.source.source, &lock_entry.source.repo, &skill);
		let lock_url = lock_entry.source.url.clone();
		insert_lock_entry(&mut skills_flake_lock.source, &key_path, lock_entry);
		repo_report
			.entry(lock_url)
			.or_default()
			.prefetched
			.push(skill.clone());
		completed += 1;
		if use_progress {
			header_span.pb_inc(1);
			header_span.pb_set_message(&render_gradient_bar(
				completed,
				total_tasks,
				terminal_bar_width()?,
			));
		} else if log_info {
			info!(skill = %skill, "prefetch.skill.done");
		}
	}

	if use_progress {
		header_span.pb_set_position(total_tasks);
		header_span.pb_set_finish_message(&render_gradient_bar(
			total_tasks,
			total_tasks,
			terminal_bar_width()?,
		));
	}

	// TODO: final report in TUI mode

	Ok(skills_flake_lock)
}

fn hierarchy_path(provider: &str, repo: &str, skill: &str) -> Vec<String> {
	let mut path = vec![provider.to_string()];
	path.extend(
		repo.split('/')
			.filter(|part| !part.is_empty())
			.map(ToString::to_string),
	);
	path.extend(
		skill
			.split('/')
			.filter(|part| !part.is_empty())
			.map(ToString::to_string),
	);
	path
}

fn insert_lock_entry(
	root: &mut BTreeMap<String, LockNode>,
	path: &[String],
	entry: SkillLockEntry,
) {
	if path.is_empty() {
		return;
	}
	if path.len() == 1 {
		root.insert(path[0].clone(), LockNode::Entry(entry));
		return;
	}

	let node = root
		.entry(path[0].clone())
		.or_insert_with(|| LockNode::Branch(BTreeMap::new()));

	match node {
		LockNode::Branch(children) => insert_lock_entry(children, &path[1..], entry),
		LockNode::Entry(_) => {
			let mut children = BTreeMap::new();
			insert_lock_entry(&mut children, &path[1..], entry);
			*node = LockNode::Branch(children);
		}
	}
}

fn terminal_bar_width() -> Result<usize> {
	let cols = crossterm::terminal::size()?.0 as usize;
	Ok(cols)
}

fn use_progress_ui() -> bool {
	std::env::var("TERM").map_or(true, |term| term != "dumb") && std::io::stderr().is_terminal()
}

fn render_gradient_bar(done: u64, total: u64, width: usize) -> String {
	if total == 0 {
		return "░".repeat(width);
	}
	let filled = ((done as f64 / total as f64) * width as f64).round() as usize;
	let mut out = String::with_capacity(width * 8);
	for i in 0..width {
		if i < filled {
			let t = if width <= 1 {
				0.0
			} else {
				i as f32 / (width - 1) as f32
			};
			let r = (64.0 + (176.0 - 64.0) * t) as u8;
			let g = (140.0 + (92.0 - 140.0) * t) as u8;
			let b = 255_u8;
			out.push_str(&"█".truecolor(r, g, b).to_string());
		} else {
			out.push_str(&"░".dimmed().to_string());
		}
	}
	out
}
