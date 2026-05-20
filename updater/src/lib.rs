use miette::Diagnostic;
use thiserror::Error;

pub mod git;
pub mod model;
pub mod parse;
pub mod update;

pub use model::{LockNode, ParsedSkillListKdl, SkillLockEntry, SkillSource, SkillsFlakeLock};
pub use parse::{parse_skill_list_kdl, sort_skill_list_kdl};
pub use update::update;

pub type Result<T> = std::result::Result<T, UpdaterError>;

#[derive(Debug, Error, Diagnostic)]
pub enum UpdaterError {
	#[error("kdl parse error: {0}")]
	KdlParse(#[from] kdl::KdlError),
	#[error("io error: {0}")]
	Io(#[from] std::io::Error),
	#[error("json error: {0}")]
	Json(#[from] serde_json::Error),
	#[error(transparent)]
	#[diagnostic(transparent)]
	InvalidSkillList(#[from] InvalidSkillListError),
	#[error("command failed ({cmd}): {stderr}")]
	CommandFailed { cmd: String, stderr: String },
	#[error("required command not found in PATH: {command}")]
	MissingCommand { command: String },
	#[error("failed to read file `{path}`: {source}")]
	ReadFile {
		path: String,
		#[source]
		source: std::io::Error,
	},
	#[error("failed to write file `{path}`: {source}")]
	WriteFile {
		path: String,
		#[source]
		source: std::io::Error,
	},
	#[error("failed to parse JSON file `{path}`: {source}")]
	ParseJsonFile {
		path: String,
		#[source]
		source: serde_json::Error,
	},
	#[error("no matching repos for filters: {filters}")]
	NoMatchingRepos { filters: String },
}

#[derive(Debug, Error, Diagnostic)]
pub enum InvalidSkillListError {
	#[error("missing `skills {{ ... }}` node")]
	MissingSkillsNode,
	#[error("`skills` node must have children")]
	SkillsNodeMustHaveChildren,
	#[error("unsupported skill source `{kind}` (expected `git` or `github`)")]
	UnsupportedSkillSource { kind: String },
	#[error("`{kind}` requires string arg")]
	MissingSourceArg { kind: String },
	#[error("`github` requires `owner/repo`, got `{value}`")]
	InvalidGithubShorthand { value: String },
	#[error("git `{url}` cannot set more than one of `branch`, `rev`, `tag`")]
	MultipleGitRefs { url: String },
	#[error("empty git URL")]
	EmptyGitUrl,
	#[error("invalid git URL `{url}`")]
	InvalidGitUrl { url: String },
	#[error("git URL must include owner/repo path: `{url}`")]
	MissingOwnerRepoInUrl { url: String },
	#[error(
		"skills source not found\n  url: {url}\n  skills_dir: {skills_dir}\n  cause: {cause}\n  fix: set correct `skills_dir` in skill-list.kdl"
	)]
	SkillsDirReadFailed {
		url: String,
		skills_dir: String,
		cause: String,
	},
	#[error(
		"invalid skill entry\n  url: {url}\n  skills_dir: {skills_dir}\n  skill: {skill}\n  missing: {skills_dir}/{skill}/SKILL.md"
	)]
	#[diagnostic(help(
		"if this folder is not a skill, add `ignore_dir=\"{skill}\"`; otherwise set `skills_dir` to actual skills root"
	))]
	MissingSkillMd {
		url: String,
		skills_dir: String,
		skill: String,
	},
	#[error(
		"requested skills not found\n  url: {url}\n  skills_dir: {skills_dir}\n  missing: {missing}\n  fix: check `skill=...` names or `skills_dir`"
	)]
	SelectedSkillsNotFound {
		url: String,
		skills_dir: String,
		missing: String,
	},
}
