use std::{collections::HashSet, fs, path::Path, process::Command, time::Duration};

use crate::{
	InvalidSkillListError, Result, UpdaterError,
	model::{PrefetchOutput, SkillSource},
};
use url::Url;

pub fn build_skill_source(
	url: String,
	branch: Option<String>,
	rev: Option<String>,
	tag: Option<String>,
	skills_dir: Option<String>,
	skills: Option<Vec<String>>,
	ignored_dirs: Option<Vec<String>>,
) -> Result<SkillSource> {
	let (source, repo) = parse_git_source_and_repo(&url)?;
	let skills_dir = match skills_dir.as_deref() {
		Some("/") => ".".to_string(),
		Some(dir) => dir.to_string(),
		None => "skills".to_string(),
	};

	Ok(SkillSource {
		source,
		url,
		repo,
		branch,
		rev,
		tag,
		skills_dir,
		skills: skills.unwrap_or_default(),
		ignored_dirs: ignored_dirs.unwrap_or_default(),
	})
}

pub fn parse_git_source_and_repo(url: &str) -> Result<(String, String)> {
	let trimmed = url.trim();
	if trimmed.is_empty() {
		return Err(UpdaterError::InvalidSkillList(
			InvalidSkillListError::EmptyGitUrl,
		));
	}

	let (host, raw_path) = if let Some(rest) = trimmed.strip_prefix("git@") {
		let (host, path) = rest.split_once(':').ok_or_else(|| {
			UpdaterError::InvalidSkillList(InvalidSkillListError::InvalidGitUrl {
				url: trimmed.to_string(),
			})
		})?;
		(host.to_string(), path.to_string())
	} else {
		let parsed = Url::parse(trimmed).map_err(|_| {
			UpdaterError::InvalidSkillList(InvalidSkillListError::InvalidGitUrl {
				url: trimmed.to_string(),
			})
		})?;
		let host = parsed.host_str().ok_or_else(|| {
			UpdaterError::InvalidSkillList(InvalidSkillListError::InvalidGitUrl {
				url: trimmed.to_string(),
			})
		})?;
		(host.to_string(), parsed.path().to_string())
	};

	let repo = raw_path
		.trim_start_matches('/')
		.trim_end_matches('/')
		.trim_end_matches(".git")
		.to_string();

	if repo.split('/').filter(|part| !part.is_empty()).count() < 2 {
		return Err(UpdaterError::InvalidSkillList(
			InvalidSkillListError::MissingOwnerRepoInUrl {
				url: trimmed.to_string(),
			},
		));
	}

	let source = match host.to_ascii_lowercase().as_str() {
		"github.com" => "github".to_string(),
		_ => host.to_lowercase(),
	};

	Ok((source, repo))
}

pub fn fetch_skill_dirs(source: &SkillSource) -> Result<(String, Vec<String>)> {
	let tmp = mktemp_dir()?;
	if source.skills_dir == "." {
		run_cmd(
			Command::new("git")
				.arg("clone")
				.arg("--depth")
				.arg("1")
				.arg(source.git_url())
				.arg(tmp.path()),
		)?;
	} else {
		run_cmd(
			Command::new("git")
				.arg("clone")
				.arg("--filter=blob:none")
				.arg("--sparse")
				.arg("--no-checkout")
				.arg(source.git_url())
				.arg(tmp.path()),
		)?;
		run_cmd(
			Command::new("git")
				.current_dir(tmp.path())
				.arg("sparse-checkout")
				.arg("init")
				.arg("--cone"),
		)?;
		run_cmd(
			Command::new("git")
				.current_dir(tmp.path())
				.arg("sparse-checkout")
				.arg("set")
				.arg(&source.skills_dir),
		)?;
	}
	if let Some(reference) = source.ref_arg() {
		run_cmd(
			Command::new("git")
				.current_dir(tmp.path())
				.arg("checkout")
				.arg(reference),
		)?;
	} else {
		run_cmd(
			Command::new("git")
				.current_dir(tmp.path())
				.arg("checkout")
				.arg("HEAD"),
		)?;
	}

	let rev_out = Command::new("git")
		.current_dir(tmp.path())
		.arg("rev-parse")
		.arg("HEAD")
		.output()
		.map_err(|err| {
			if err.kind() == std::io::ErrorKind::NotFound {
				UpdaterError::MissingCommand {
					command: "git".to_string(),
				}
			} else {
				UpdaterError::Io(err)
			}
		})?;
	if !rev_out.status.success() {
		return Err(UpdaterError::CommandFailed {
			cmd: "git rev-parse HEAD".to_string(),
			stderr: String::from_utf8_lossy(&rev_out.stderr).trim().to_string(),
		});
	}
	let resolved_rev = String::from_utf8_lossy(&rev_out.stdout).trim().to_string();

	let skills_root = if source.skills_dir == "." {
		tmp.path().to_path_buf()
	} else {
		tmp.path().join(&source.skills_dir)
	};
	let mut skills = Vec::new();
	let selected = source.skills.iter().cloned().collect::<HashSet<_>>();
	let mut seen_selected = HashSet::new();
	collect_skills_recursive(
		&skills_root,
		&skills_root,
		source,
		&selected,
		&mut seen_selected,
		&mut skills,
	)?;

	if !selected.is_empty() {
		let missing = selected
			.difference(&seen_selected)
			.cloned()
			.collect::<Vec<_>>();
		if !missing.is_empty() {
			return Err(UpdaterError::InvalidSkillList(
				InvalidSkillListError::SelectedSkillsNotFound {
					url: source.url.clone(),
					skills_dir: source.skills_dir.clone(),
					missing: missing.join(", "),
				},
			));
		}
	}
	skills.sort();
	Ok((resolved_rev, skills))
}

pub fn prefetch_skill(source: &SkillSource, skill: &str) -> Result<PrefetchOutput> {
	let sparse_path = if source.skills_dir == "." {
		skill.to_string()
	} else {
		format!("{}/{}", source.skills_dir, skill)
	};
	let mut cmd = Command::new("nix-prefetch-git");
	cmd.arg("--quiet")
		.arg("--url")
		.arg(source.git_url())
		.arg("--sparse-checkout")
		.arg(&sparse_path)
		.arg("--root-dir")
		.arg(&sparse_path);
	if let Some(rev) = source.rev.as_deref() {
		cmd.arg("--rev").arg(rev);
	} else if let Some(tag) = source.tag.as_deref() {
		cmd.arg("--rev").arg(tag);
	} else if let Some(branch) = source.branch.as_deref() {
		cmd.arg("--branch-name").arg(branch);
	}

	let out = cmd.output().map_err(|err| {
		if err.kind() == std::io::ErrorKind::NotFound {
			UpdaterError::MissingCommand {
				command: "nix-prefetch-git".to_string(),
			}
		} else {
			UpdaterError::Io(err)
		}
	})?;
	if !out.status.success() {
		cleanup_git_checkout_err_dirs(Duration::from_secs(60 * 10));
		return Err(UpdaterError::CommandFailed {
			cmd: "nix-prefetch-git".to_string(),
			stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
		});
	}
	Ok(serde_json::from_slice::<PrefetchOutput>(&out.stdout)?)
}

fn run_cmd(cmd: &mut Command) -> Result<()> {
	let display = format!("{:?}", cmd);
	let out = cmd.output().map_err(|err| {
		if err.kind() == std::io::ErrorKind::NotFound {
			UpdaterError::MissingCommand {
				command: display.clone(),
			}
		} else {
			UpdaterError::Io(err)
		}
	})?;
	if out.status.success() {
		return Ok(());
	}
	Err(UpdaterError::CommandFailed {
		cmd: display,
		stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
	})
}

fn mktemp_dir() -> Result<tempfile::TempDir> {
	tempfile::Builder::new()
		.prefix("skills-flake-updater-")
		.tempdir()
		.map_err(UpdaterError::Io)
}

fn cleanup_git_checkout_err_dirs(max_age: Duration) {
	let tmp = std::env::temp_dir();
	let Ok(entries) = fs::read_dir(tmp) else {
		return;
	};
	let now = std::time::SystemTime::now();
	for entry in entries.flatten() {
		let path = entry.path();
		let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
			continue;
		};
		if !name.starts_with("git-checkout-err-") {
			continue;
		}
		let Ok(meta) = entry.metadata() else {
			continue;
		};
		let Ok(modified) = meta.modified() else {
			continue;
		};
		let Ok(age) = now.duration_since(modified) else {
			continue;
		};
		if age >= max_age {
			let _ = fs::remove_dir_all(&path);
		}
	}
}

fn collect_skills_recursive(
	root: &Path,
	dir: &Path,
	source: &SkillSource,
	selected: &HashSet<String>,
	seen_selected: &mut HashSet<String>,
	out: &mut Vec<String>,
) -> Result<()> {
	for entry in fs::read_dir(dir).map_err(|err| {
		UpdaterError::InvalidSkillList(InvalidSkillListError::SkillsDirReadFailed {
			url: source.url.clone(),
			skills_dir: source.skills_dir.clone(),
			cause: err.to_string(),
		})
	})? {
		let entry = entry?;
		let path = entry.path();
		if !path.is_dir() {
			continue;
		}
		let name = entry.file_name().to_string_lossy().to_string();
		if name.starts_with('.') || source.ignored_dirs.iter().any(|d| d == &name) {
			continue;
		}

		let rel = match path.strip_prefix(root) {
			Ok(p) => p,
			Err(_) => continue,
		};
		let rel_name = rel.to_string_lossy().replace('\\', "/");
		let has_skill_md = path.join("SKILL.md").exists();

		if has_skill_md {
			if selected.is_empty() || selected.contains(&rel_name) {
				seen_selected.insert(rel_name.clone());
				out.push(rel_name);
			}
			continue;
		}

		collect_skills_recursive(root, &path, source, selected, seen_selected, out)?;
	}
	Ok(())
}
