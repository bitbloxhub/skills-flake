use crate::{
	InvalidSkillListError, Result, UpdaterError, git::build_skill_source, model::ParsedSkillListKdl,
};

pub fn parse_skill_list_kdl(input: &str) -> Result<ParsedSkillListKdl> {
	let document = input.parse::<kdl::KdlDocument>()?;
	let skills_node = document
		.get("skills")
		.ok_or(UpdaterError::InvalidSkillList(
			InvalidSkillListError::MissingSkillsNode,
		))?;
	let children = skills_node
		.children()
		.ok_or(UpdaterError::InvalidSkillList(
			InvalidSkillListError::SkillsNodeMustHaveChildren,
		))?;

	let mut skills = Vec::new();
	for node in children.nodes() {
		let source_kind = node.name().value();
		if source_kind != "git" && source_kind != "github" {
			return Err(UpdaterError::InvalidSkillList(
				InvalidSkillListError::UnsupportedSkillSource {
					kind: source_kind.to_string(),
				},
			));
		}

		let source_arg = node
			.entries()
			.iter()
			.find(|entry| entry.name().is_none())
			.and_then(|entry| entry.value().as_string())
			.ok_or_else(|| {
				UpdaterError::InvalidSkillList(InvalidSkillListError::MissingSourceArg {
					kind: source_kind.to_string(),
				})
			})?
			.to_string();
		let url = if source_kind == "github" {
			let trimmed = source_arg.trim();
			let parts = trimmed.split('/').collect::<Vec<_>>();
			if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
				return Err(UpdaterError::InvalidSkillList(
					InvalidSkillListError::InvalidGithubShorthand {
						value: source_arg.clone(),
					},
				));
			}
			format!("https://github.com/{trimmed}.git")
		} else {
			source_arg
		};
		let branch = node
			.get("branch")
			.and_then(|value| value.as_string())
			.map(ToString::to_string);
		let rev = node
			.get("rev")
			.and_then(|value| value.as_string())
			.map(ToString::to_string);
		let tag = node
			.get("tag")
			.and_then(|value| value.as_string())
			.map(ToString::to_string);
		let skills_dir = node
			.get("skills_dir")
			.and_then(|value| value.as_string())
			.map(ToString::to_string);
		let mut selected_skills = node
			.entries()
			.iter()
			.filter_map(|entry| match (entry.name(), entry.value().as_string()) {
				(Some(name), Some(value)) if name.value() == "skill" => Some(value.to_string()),
				_ => None,
			})
			.collect::<Vec<_>>();
		if let Some(skills_csv) = node.get("skills").and_then(|value| value.as_string()) {
			selected_skills.extend(
				skills_csv
					.split(',')
					.map(str::trim)
					.filter(|s| !s.is_empty())
					.map(ToString::to_string),
			);
		}
		selected_skills.sort();
		selected_skills.dedup();
		let mut ignored_dirs = node
			.entries()
			.iter()
			.filter_map(|entry| match (entry.name(), entry.value().as_string()) {
				(Some(name), Some(value)) if name.value() == "ignore_dir" => {
					Some(value.to_string())
				}
				_ => None,
			})
			.collect::<Vec<_>>();
		if let Some(ignored_csv) = node.get("ignored_dirs").and_then(|value| value.as_string()) {
			ignored_dirs.extend(
				ignored_csv
					.split(',')
					.map(str::trim)
					.filter(|s| !s.is_empty())
					.map(ToString::to_string),
			);
		}
		ignored_dirs.sort();
		ignored_dirs.dedup();

		let ref_count = [branch.is_some(), rev.is_some(), tag.is_some()]
			.into_iter()
			.filter(|flag| *flag)
			.count();
		if ref_count > 1 {
			return Err(UpdaterError::InvalidSkillList(
				InvalidSkillListError::MultipleGitRefs { url },
			));
		}

		skills.push(build_skill_source(
			url,
			branch,
			rev,
			tag,
			skills_dir,
			if selected_skills.is_empty() {
				None
			} else {
				Some(selected_skills)
			},
			if ignored_dirs.is_empty() {
				None
			} else {
				Some(ignored_dirs)
			},
		)?);
	}

	Ok(ParsedSkillListKdl { skills })
}

pub fn sort_skill_list_kdl(input: &str) -> Result<String> {
	let document = input.parse::<kdl::KdlDocument>()?;
	let skills_node = document
		.get("skills")
		.ok_or(UpdaterError::InvalidSkillList(
			InvalidSkillListError::MissingSkillsNode,
		))?;
	let children = skills_node
		.children()
		.ok_or(UpdaterError::InvalidSkillList(
			InvalidSkillListError::SkillsNodeMustHaveChildren,
		))?;

	let mut nodes = children.nodes().iter().cloned().collect::<Vec<_>>();
	nodes.sort_by(|a, b| {
		let a_url = a
			.entries()
			.iter()
			.find(|entry| entry.name().is_none())
			.and_then(|entry| entry.value().as_string())
			.unwrap_or("");
		let b_url = b
			.entries()
			.iter()
			.find(|entry| entry.name().is_none())
			.and_then(|entry| entry.value().as_string())
			.unwrap_or("");
		a_url.cmp(b_url)
	});

	let mut out = String::from("skills {\n");
	for node in nodes {
		let line = node
			.to_string()
			.lines()
			.map(str::trim)
			.filter(|part| !part.is_empty())
			.collect::<Vec<_>>()
			.join(" ");
		out.push('\t');
		out.push_str(&line);
		out.push('\n');
	}
	out.push_str("}\n");
	Ok(out)
}
