use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};

pub const DEFAULT_SKILL: &str = "kten";

const KTEN_SKILL: &str = include_str!("skills/kten/SKILL.md");
const KTEN_MCP_SKILL: &str = include_str!("skills/kten-mcp/SKILL.md");

#[derive(Debug, Clone, Copy)]
pub struct BundledSkill {
    pub name: &'static str,
    pub source: &'static str,
    pub description: &'static str,
    pub contents: &'static str,
}

#[derive(Debug)]
pub enum InstallScope {
    Project,
    Global,
    Custom(PathBuf),
}

#[derive(Debug, PartialEq, Eq)]
pub enum InstallOutcome {
    Installed(PathBuf),
    Overwrote(PathBuf),
    AlreadyExists(PathBuf),
}

pub fn find_skill(name: &str) -> anyhow::Result<BundledSkill> {
    list_skills()
        .iter()
        .copied()
        .find(|skill| skill.name == name)
        .with_context(|| {
            format!(
                "Skill not found: unknown skill \"{name}\". Run 'kten skills list' to see available skills."
            )
        })
}

pub fn list_skills() -> &'static [BundledSkill] {
    &[
        BundledSkill {
            name: "kten",
            source: "bundled",
            description: "Kaiten CLI workflow for AI agents.",
            contents: KTEN_SKILL,
        },
        BundledSkill {
            name: "kten-mcp",
            source: "bundled",
            description: "Configure and use the kten stdio MCP server for Kaiten access.",
            contents: KTEN_MCP_SKILL,
        },
    ]
}

pub fn install_skill(
    skill: BundledSkill,
    scope: InstallScope,
    force: bool,
) -> anyhow::Result<InstallOutcome> {
    let root = target_root(scope)?;
    write_skill(skill, &root, force)
}

fn write_skill(skill: BundledSkill, root: &Path, force: bool) -> anyhow::Result<InstallOutcome> {
    let skill_dir = root.join(skill.name);
    let skill_file = skill_dir.join("SKILL.md");

    if skill_file.exists() && !force {
        return Ok(InstallOutcome::AlreadyExists(skill_file));
    }

    let overwriting = skill_file.exists();
    fs::create_dir_all(&skill_dir)
        .with_context(|| format!("failed to create skill directory {}", skill_dir.display()))?;
    fs::write(&skill_file, skill.contents)
        .with_context(|| format!("failed to write {}", skill_file.display()))?;

    if overwriting {
        Ok(InstallOutcome::Overwrote(skill_dir))
    } else {
        Ok(InstallOutcome::Installed(skill_dir))
    }
}

fn target_root(scope: InstallScope) -> anyhow::Result<PathBuf> {
    match scope {
        InstallScope::Project => Ok(find_git_root()?.join(".agents").join("skills")),
        InstallScope::Global => Ok(home_dir()?.join(".agents").join("skills")),
        InstallScope::Custom(path) => Ok(path),
    }
}

fn find_git_root() -> anyhow::Result<PathBuf> {
    let cwd = env::current_dir().context("failed to read current directory")?;
    for ancestor in cwd.ancestors() {
        if ancestor.join(".git").exists() {
            return Ok(ancestor.to_path_buf());
        }
    }
    bail!("not inside a git repository; pass --path or --global")
}

fn home_dir() -> anyhow::Result<PathBuf> {
    if let Some(home) = env::var_os("HOME") {
        return Ok(PathBuf::from(home));
    }
    dirs::home_dir().context("failed to resolve home directory")
}
