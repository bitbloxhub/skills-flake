use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ParsedSkillListKdl {
	pub skills: Vec<SkillSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillSource {
	pub source: String,
	pub url: String,
	pub repo: String,
	pub branch: Option<String>,
	pub rev: Option<String>,
	pub tag: Option<String>,
	pub skills_dir: String,
	#[serde(default)]
	pub skills: Vec<String>,
	#[serde(default)]
	pub ignored_dirs: Vec<String>,
}

impl SkillSource {
	pub fn ref_arg(&self) -> Option<&str> {
		self.rev
			.as_deref()
			.or(self.tag.as_deref())
			.or(self.branch.as_deref())
	}

	pub fn git_url(&self) -> String {
		self.url.clone()
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillsFlakeLock {
	pub version: u32,
	pub source: BTreeMap<String, LockNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LockNode {
	Branch(BTreeMap<String, LockNode>),
	Entry(SkillLockEntry),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillLockEntry {
	pub hash: String,
	pub source: SkillSource,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrefetchOutput {
	pub rev: String,
	pub hash: String,
}
