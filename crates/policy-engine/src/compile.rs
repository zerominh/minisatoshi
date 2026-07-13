use crate::config::PolicyConfig;
use crate::error::PolicyError;
use crate::parser::{self, Expr};
use crate::timelock::parse_duration;
use crate::translate::translate_policy_keys;
use crate::validate::validate;
use miniscript::policy::Concrete;
use miniscript::DescriptorPublicKey;

/// Compile a validated policy configuration into an abstract Miniscript policy string.
/// Keys remain as logical identifiers (A, B, C) for debugging and display.
pub fn compile_abstract_policy_string(config: &PolicyConfig) -> Result<String, PolicyError> {
    validate(config)?;
    build_abstract_policy_string(config)
}

/// Compile a validated policy configuration into a descriptor-ready Miniscript policy string.
pub fn compile_policy_string(config: &PolicyConfig) -> Result<String, PolicyError> {
    let abstract_policy = compile_abstract_policy_string(config)?;
    let policy: Concrete<String> = abstract_policy
        .parse::<Concrete<String>>()
        .map_err(|e| PolicyError::MiniscriptCompile(e.to_string()))?;

    let translated = translate_policy_keys(&policy, config)?;
    Ok(translated.to_string())
}

/// Compile a validated policy configuration into an abstract Miniscript policy AST.
pub fn compile_abstract_miniscript(
    config: &PolicyConfig,
) -> Result<Concrete<String>, PolicyError> {
    let abstract_policy = compile_abstract_policy_string(config)?;
    abstract_policy
        .parse::<Concrete<String>>()
        .map_err(|e| PolicyError::MiniscriptCompile(e.to_string()))
}

/// Compile a validated policy configuration into a typed Miniscript policy AST.
pub fn compile_miniscript(
    config: &PolicyConfig,
) -> Result<Concrete<DescriptorPublicKey>, PolicyError> {
    let abstract_policy = compile_abstract_policy_string(config)?;
    let policy: Concrete<String> = abstract_policy
        .parse::<Concrete<String>>()
        .map_err(|e| PolicyError::MiniscriptCompile(e.to_string()))?;

    translate_policy_keys(&policy, config)
}

/// Decompose a policy into disjoint tapscript leaves (top-level OR branches + fallback).
pub fn compile_leaf_policies(config: &PolicyConfig) -> Result<Vec<String>, PolicyError> {
    validate(config)?;
    let primary_ast = parser::parse_expression(&config.policy.primary)?;
    let mut leaves = Vec::new();
    collect_or_leaves(&primary_ast, &mut leaves)?;

    if let Some(fallback) = &config.policy.fallback {
        let blocks = parse_duration(&fallback.after)?;
        leaves.push(format!("and(older({blocks}),pk({}))", fallback.allow));
    }

    Ok(leaves)
}

fn collect_or_leaves(ast: &Expr, leaves: &mut Vec<String>) -> Result<(), PolicyError> {
    match ast {
        Expr::Or(left, right) => {
            collect_or_leaves(left, leaves)?;
            collect_or_leaves(right, leaves)?;
        }
        _ => leaves.push(ast_to_abstract_policy(ast)?),
    }
    Ok(())
}
fn build_abstract_policy_string(config: &PolicyConfig) -> Result<String, PolicyError> {
    let primary_ast = parser::parse_expression(&config.policy.primary)?;
    let primary = ast_to_abstract_policy(&primary_ast)?;

    let full = if let Some(fallback) = &config.policy.fallback {
        let blocks = parse_duration(&fallback.after)?;
        format!(
            "or({primary},and(older({blocks}),pk({})))",
            fallback.allow
        )
    } else {
        primary
    };

    Ok(full)
}

fn ast_to_abstract_policy(ast: &Expr) -> Result<String, PolicyError> {
    match ast {
        Expr::Key(id) => Ok(format!("pk({id})")),
        Expr::And(left, right) => {
            let left = ast_to_abstract_policy(left)?;
            let right = ast_to_abstract_policy(right)?;
            Ok(format!("and({left},{right})"))
        }
        Expr::Or(left, right) => {
            let left = ast_to_abstract_policy(left)?;
            let right = ast_to_abstract_policy(right)?;
            Ok(format!("or({left},{right})"))
        }
    }
}
