//! Skill tool handler factory.

use runtime::AgentScope;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};
use wcore::{
    ToolDispatch, ToolHandler,
    agent::{AsTool, ToolDescription},
    model::Tool,
    repos::Storage,
};

#[derive(Deserialize, schemars::JsonSchema)]
pub struct SkillTool {
    /// Skill name to load. If no exact match, returns fuzzy matches.
    /// Leave empty to list all available skills.
    pub name: String,
}

impl ToolDescription for SkillTool {
    const DESCRIPTION: &'static str = "Load a skill by name. Returns its instructions on exact match, or lists matching skills otherwise.";
}

pub fn handler<S: Storage + 'static>(
    storage: Arc<S>,
    scopes: Arc<RwLock<BTreeMap<String, AgentScope>>>,
) -> (Tool, ToolHandler) {
    (
        SkillTool::as_tool(),
        Arc::new(move |call: ToolDispatch| {
            let storage = storage.clone();
            let scopes = scopes.clone();
            Box::pin(async move {
                let input: SkillTool = serde_json::from_str(&call.args)
                    .map_err(|e| format!("invalid arguments: {e}"))?;
                let name = &input.name;

                // Enforce skill scope.
                {
                    let scopes = scopes.read().expect("scopes lock poisoned");
                    if let Some(scope) = scopes.get(&call.agent)
                        && !scope.skills.is_empty()
                        && !scope.skills.iter().any(|s| s == name)
                    {
                        return Err(format!("skill not available: {name}"));
                    }
                }

                // Guard against path traversal.
                if name.contains("..") || name.contains('/') || name.contains('\\') {
                    return Err(format!("invalid skill name: {name}"));
                }

                // Try exact load.
                if !name.is_empty() {
                    match storage.load_skill(name) {
                        Ok(Some(skill)) => return Ok(skill.body),
                        Ok(None) => {}
                        Err(e) => return Err(format!("failed to load skill: {e}")),
                    }
                }

                // No exact match — fuzzy search / list all.
                let query = name.to_lowercase();
                let allowed: Vec<String> = scopes
                    .read()
                    .expect("scopes lock poisoned")
                    .get(&call.agent)
                    .map(|s| s.skills.clone())
                    .unwrap_or_default();

                let skills = storage.list_skills().unwrap_or_default();
                let matches: Vec<String> = skills
                    .iter()
                    .filter(|s| {
                        if !allowed.is_empty() && !allowed.iter().any(|a| a == s.name.as_str()) {
                            return false;
                        }
                        query.is_empty()
                            || s.name.to_lowercase().contains(&query)
                            || s.description.to_lowercase().contains(&query)
                    })
                    .map(|s| format!("{}: {}", s.name, s.description))
                    .collect();

                if matches.is_empty() {
                    Ok("no skills found".to_owned())
                } else {
                    Ok(matches.join("\n"))
                }
            })
        }),
    )
}
