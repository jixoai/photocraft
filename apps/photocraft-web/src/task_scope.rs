use serde_json::{Value, json};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scope(Option<String>);

impl Scope {
    pub fn new(id: Option<String>) -> Result<Self, String> {
        if let Some(id) = &id
            && (id.is_empty() || id.len() > 64 || !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')))
        {
            return Err("Task scope must contain 1–64 ASCII letters, digits, underscores or hyphens".into());
        }
        Ok(Self(id))
    }

    pub fn id(&self) -> Option<&str> {
        self.0.as_deref()
    }

    pub fn storage(&self, legacy: &str) -> String {
        self.id().map_or_else(|| legacy.to_owned(), |id| format!("{legacy}.scope.{id}"))
    }

    pub fn authorize(&self, tool: &str, args: &Value) -> Result<(), crate::agent_error::Failure> {
        use crate::agent_error::Failure;
        let supplied = args.get("scopeId").and_then(Value::as_str);
        if let Some(supplied) = supplied {
            Self::new(Some(supplied.into())).map_err(|message| Failure::invalid("/scopeId", json!({"pattern":"^[A-Za-z0-9_-]+$"}), message))?;
            if Some(supplied) != self.id() {
                return Err(Failure::rejected(
                    "SCOPE_MISMATCH",
                    "This page belongs to a different task scope",
                    "Use the assigned task page; do not change scopeId to fit an unexpected tab.",
                ));
            }
        }
        let discovery = matches!(tool, "photocraft_commands" | "photocraft_inspect")
            || (tool == "photocraft_control"
                && matches!(
                    args["method"].as_str(),
                    Some(
                        "scope.status"
                            | "requests.status"
                            | "recovery.list"
                            | "engine.commands"
                            | "ui.inspect"
                            | "ui.menu.list"
                            | "ui.screenshot"
                            | "jobs.list"
                    )
                ));
        if self.id().is_some() && supplied.is_none() && !discovery {
            return Err(Failure::rejected(
                "SCOPE_REQUIRED",
                "Explicit task scopeId is required on this page",
                "Discover scope.status on the assigned page, verify its identity, then include that scopeId.",
            ));
        }
        Ok(())
    }

    pub fn status(&self) -> Value {
        json!({
            "scopeId":self.id(),
            "isolated":self.id().is_some(),
            "singleWriter":self.id().is_some(),
            "storage":"origin-local",
            "securityBoundary":false
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_storage_names_remain_unchanged() {
        let scope = Scope::new(None).unwrap();
        for key in ["photocraft.recovery", "photocraft.presets", "photocraft.preferences"] {
            assert_eq!(scope.storage(key), key);
        }
        assert_eq!(scope.status()["isolated"], false);
    }

    #[test]
    fn task_namespaces_are_disjoint_and_stable() {
        let first = Scope::new(Some("task-A_7".into())).unwrap();
        let second = Scope::new(Some("task-B_7".into())).unwrap();
        for key in ["photocraft.recovery", "photocraft.presets", "photocraft.preferences", "photocraft.writer"] {
            assert_ne!(first.storage(key), second.storage(key));
            assert_ne!(first.storage(key), key);
            assert_eq!(first.storage(key), Scope::new(Some("task-A_7".into())).unwrap().storage(key));
        }
        assert_eq!(first.status()["scopeId"], "task-A_7");
        assert_eq!(first.status()["securityBoundary"], false);
    }

    #[test]
    fn malformed_scope_never_falls_back_to_shared_storage() {
        for id in ["", ".", "../task", "a/b", "a.b", "with space", "任务", "a%2fb", "a\0b", &"a".repeat(65)] {
            assert!(Scope::new(Some(id.into())).is_err(), "{id:?}");
        }
        assert!(Scope::new(Some("a".repeat(64))).is_ok());
    }
}
