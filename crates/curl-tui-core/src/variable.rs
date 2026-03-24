use crate::types::Variable;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

const MAX_RESOLVE_DEPTH: usize = 10;

#[derive(Debug, Error, PartialEq)]
pub enum ResolveError {
    #[error("Undefined variables: {0:?}")]
    UndefinedVariables(Vec<String>),
    #[error("Circular variable reference detected: {0}")]
    CircularReference(String),
}

/// Resolves `{{variable}}` placeholders using a three-tier hierarchy.
pub struct FileVariableResolver {
    /// Merged variables: collection > environment > global
    variables: HashMap<String, Variable>,
}

impl FileVariableResolver {
    /// Create a new resolver. Pass `None` for layers that don't apply.
    pub fn new(
        global: HashMap<String, Variable>,
        environment: Option<HashMap<String, Variable>>,
        collection: Option<HashMap<String, Variable>>,
    ) -> Self {
        let mut merged = global;
        if let Some(env_vars) = environment {
            merged.extend(env_vars);
        }
        if let Some(col_vars) = collection {
            merged.extend(col_vars);
        }
        Self { variables: merged }
    }

    /// Resolve all `{{var}}` placeholders in `input`.
    ///
    /// Returns `(resolved_string, secret_values)` where `secret_values` contains
    /// the raw values of any secret variables that were substituted.
    pub fn resolve(&self, input: &str) -> Result<(String, Vec<String>), ResolveError> {
        let mut secrets = Vec::new();
        let mut visiting = HashSet::new();
        let result = self.resolve_inner(input, &mut secrets, &mut visiting, 0)?;
        Ok((result, secrets))
    }

    fn resolve_inner(
        &self,
        input: &str,
        secrets: &mut Vec<String>,
        visiting: &mut HashSet<String>,
        depth: usize,
    ) -> Result<String, ResolveError> {
        if depth > MAX_RESOLVE_DEPTH {
            let chain: Vec<String> = visiting.iter().cloned().collect();
            return Err(ResolveError::CircularReference(chain.join(" -> ")));
        }

        let mut result = String::new();
        let mut remaining = input;
        let mut undefined = Vec::new();

        while let Some(start) = remaining.find("{{") {
            result.push_str(&remaining[..start]);
            let after_open = &remaining[start + 2..];

            if let Some(end) = after_open.find("}}") {
                let var_name = &after_open[..end];

                if visiting.contains(var_name) {
                    let mut chain: Vec<String> = visiting.iter().cloned().collect();
                    chain.push(var_name.to_string());
                    return Err(ResolveError::CircularReference(chain.join(" -> ")));
                }

                if let Some(variable) = self.variables.get(var_name) {
                    visiting.insert(var_name.to_string());
                    let resolved =
                        self.resolve_inner(&variable.value, secrets, visiting, depth + 1)?;
                    visiting.remove(var_name);

                    if variable.secret {
                        secrets.push(resolved.clone());
                    }
                    result.push_str(&resolved);
                } else {
                    undefined.push(var_name.to_string());
                }

                remaining = &after_open[end + 2..];
            } else {
                // No closing }}, treat as literal
                result.push_str("{{");
                remaining = after_open;
            }
        }

        result.push_str(remaining);

        if !undefined.is_empty() {
            return Err(ResolveError::UndefinedVariables(undefined));
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Variable;
    use std::collections::HashMap;

    fn make_vars(pairs: &[(&str, &str, bool)]) -> HashMap<String, Variable> {
        pairs
            .iter()
            .map(|(k, v, s)| {
                (
                    k.to_string(),
                    Variable {
                        value: v.to_string(),
                        secret: *s,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn test_resolve_simple_variable() {
        let global = make_vars(&[("base_url", "https://api.example.com", false)]);
        let resolver = FileVariableResolver::new(global, None, None);
        let (result, _) = resolver.resolve("{{base_url}}/users").unwrap();
        assert_eq!(result, "https://api.example.com/users");
    }

    #[test]
    fn test_resolve_no_variables() {
        let resolver = FileVariableResolver::new(HashMap::new(), None, None);
        let (result, _) = resolver.resolve("plain text").unwrap();
        assert_eq!(result, "plain text");
    }

    #[test]
    fn test_resolve_collection_overrides_environment() {
        let global = HashMap::new();
        let env_vars = make_vars(&[("url", "http://env.com", false)]);
        let col_vars = make_vars(&[("url", "http://collection.com", false)]);
        let resolver = FileVariableResolver::new(global, Some(env_vars), Some(col_vars));
        let (result, _) = resolver.resolve("{{url}}").unwrap();
        assert_eq!(result, "http://collection.com");
    }

    #[test]
    fn test_resolve_environment_overrides_global() {
        let global = make_vars(&[("url", "http://global.com", false)]);
        let env_vars = make_vars(&[("url", "http://env.com", false)]);
        let resolver = FileVariableResolver::new(global, Some(env_vars), None);
        let (result, _) = resolver.resolve("{{url}}").unwrap();
        assert_eq!(result, "http://env.com");
    }

    #[test]
    fn test_resolve_tracks_secrets() {
        let global = make_vars(&[("token", "secret123", true)]);
        let resolver = FileVariableResolver::new(global, None, None);
        let (result, secrets) = resolver.resolve("Bearer {{token}}").unwrap();
        assert_eq!(result, "Bearer secret123");
        assert_eq!(secrets, vec!["secret123".to_string()]);
    }

    #[test]
    fn test_resolve_undefined_variable() {
        let resolver = FileVariableResolver::new(HashMap::new(), None, None);
        let result = resolver.resolve("{{undefined}}");
        assert!(matches!(result, Err(ResolveError::UndefinedVariables(_))));
        if let Err(ResolveError::UndefinedVariables(vars)) = result {
            assert_eq!(vars, vec!["undefined".to_string()]);
        }
    }

    #[test]
    fn test_resolve_multiple_variables() {
        let global = make_vars(&[("host", "example.com", false), ("port", "8080", false)]);
        let resolver = FileVariableResolver::new(global, None, None);
        let (result, _) = resolver.resolve("http://{{host}}:{{port}}/api").unwrap();
        assert_eq!(result, "http://example.com:8080/api");
    }

    #[test]
    fn test_resolve_circular_reference() {
        let global = make_vars(&[("a", "{{b}}", false), ("b", "{{a}}", false)]);
        let resolver = FileVariableResolver::new(global, None, None);
        let result = resolver.resolve("{{a}}");
        assert!(matches!(result, Err(ResolveError::CircularReference(_))));
    }

    #[test]
    fn test_resolve_nested_variable() {
        let global = make_vars(&[
            ("greeting", "hello {{name}}", false),
            ("name", "world", false),
        ]);
        let resolver = FileVariableResolver::new(global, None, None);
        let (result, _) = resolver.resolve("{{greeting}}").unwrap();
        assert_eq!(result, "hello world");
    }
}
