//! Skills, one `SKILL.md` each.
//!
//! A listing reads the markdown it summarises. The keyed store splits
//! the summary out so an enumeration never touches a body; here the body
//! *is* the file, and a second document beside it would be a copy to
//! keep in step for a catalogue one person installed by hand.

use crate::backend::{self, Backend};
use anyhow::Result;
use std::str::FromStr;
use store::{Skill, SkillSummary, interface::Skills};

impl Backend {
    fn skill_path(&self, name: &str) -> std::path::PathBuf {
        self.skills_dir()
            .join(format!("{}.md", backend::encode(name)))
    }
}

impl Skills for Backend {
    async fn list_skills(&self, limit: usize, offset: usize) -> Result<Vec<SkillSummary>> {
        let names = backend::names_in(&self.skills_dir(), ".md").await?;
        let mut out = Vec::new();
        for name in names.iter().skip(offset).take(limit) {
            if let Some(skill) = self.load_skill(name).await? {
                out.push(SkillSummary::from(&skill));
            }
        }
        Ok(out)
    }

    async fn load_skill(&self, name: &str) -> Result<Option<Skill>> {
        let Ok(markdown) = tokio::fs::read_to_string(self.skill_path(name)).await else {
            return Ok(None);
        };
        Ok(Some(Skill::from_str(&markdown)?))
    }

    /// The markdown is what is kept, so the name cannot disagree with the
    /// frontmatter it came from.
    async fn put_skill(&self, markdown: &str) -> Result<SkillSummary> {
        let skill = Skill::from_str(markdown)?;
        tokio::fs::write(self.skill_path(&skill.name), markdown).await?;
        Ok(SkillSummary::from(&skill))
    }

    async fn remove_skill(&self, name: &str) -> Result<bool> {
        Ok(tokio::fs::remove_file(self.skill_path(name)).await.is_ok())
    }
}
