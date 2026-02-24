use anyhow::Result;

use crate::util::{
    exec::{command_exists, run_command_inherit},
    output::{print_error, print_header, print_success},
};

const WORKSPACES: &[&str] = &[
    "soul-player-desktop",
    "@soul-player/shared",
    "@soul-player/marketing",
];

/// Run TypeScript type checking
pub fn run_typescript(workspace: Option<String>) -> Result<()> {
    print_header("TypeScript Type Checking");

    // Ensure yarn is available
    if !command_exists("yarn") {
        print_error("yarn not found");
        anyhow::bail!("yarn is required. Run 'corepack enable' or install Node.js");
    }

    let workspaces_to_check = if let Some(ws) = workspace {
        vec![ws]
    } else {
        WORKSPACES.iter().map(|s| s.to_string()).collect()
    };

    for ws in &workspaces_to_check {
        let friendly_name = ws.trim_start_matches("@soul-player/");

        println!("\n  Checking {} workspace...", friendly_name);

        let args = vec!["workspace", ws.as_str(), "run", "tsc", "--noEmit"];

        let success = run_command_inherit(
            "yarn",
            &args,
            &format!("TypeScript check - {}", friendly_name),
        )?;

        if !success {
            print_error(&format!("{} TypeScript check failed", friendly_name));
            anyhow::bail!("TypeScript check failed for {}", ws);
        }

        print_success(&format!("{} TypeScript OK", friendly_name));
    }

    println!();
    print_success("All TypeScript checks passed");
    Ok(())
}

/// Run ESLint
pub fn run_lint(fix: bool, workspace: Option<String>) -> Result<()> {
    print_header("ESLint");

    // Ensure yarn is available
    if !command_exists("yarn") {
        print_error("yarn not found");
        anyhow::bail!("yarn is required. Run 'corepack enable' or install Node.js");
    }

    // Only desktop and shared have ESLint configured
    let lint_workspaces = ["soul-player-desktop", "@soul-player/shared"];

    let workspaces_to_lint = if let Some(ws) = workspace {
        if !lint_workspaces.contains(&ws.as_str()) {
            print_error(&format!("Workspace {} does not have ESLint configured", ws));
            anyhow::bail!("Invalid workspace for linting");
        }
        vec![ws]
    } else {
        lint_workspaces.iter().map(|s| s.to_string()).collect()
    };

    for ws in &workspaces_to_lint {
        let friendly_name = ws.trim_start_matches("@soul-player/");

        println!("\n  Linting {} workspace...", friendly_name);

        let mut args = vec!["workspace", ws.as_str(), "run", "lint"];

        if fix {
            args.push("--fix");
        }

        let success = run_command_inherit("yarn", &args, &format!("ESLint - {}", friendly_name))?;

        if !success {
            print_error(&format!("{} ESLint failed", friendly_name));
            if !fix {
                println!("\n  Run 'cargo xtask check lint --fix' to auto-fix issues");
            }
            anyhow::bail!("ESLint failed for {}", ws);
        }

        print_success(&format!("{} ESLint OK", friendly_name));
    }

    println!();
    print_success("All ESLint checks passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_list() {
        // Verify workspaces are defined
        assert!(!WORKSPACES.is_empty());
        assert!(WORKSPACES.contains(&"soul-player-desktop"));
        assert!(WORKSPACES.contains(&"@soul-player/shared"));
    }
}
