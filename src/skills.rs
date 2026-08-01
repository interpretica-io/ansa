//! The `install-skill` command: embedded copies of the `skills/` directory,
//! written out so agents can be taught the bus without cloning the repo.

use std::io;
use std::path::{Path, PathBuf};

const CLAUDE_SKILL: &str = include_str!("../skills/claude/ansa-bus/SKILL.md");
const CODEX_SKILL: &str = include_str!("../skills/codex/ansa-bus/SKILL.md");
const CHATGPT_INSTRUCTIONS: &str = include_str!("../skills/chatgpt/instructions.md");
const CHATGPT_OPENAPI: &str = include_str!("../skills/chatgpt/ansa.openapi.yaml");

const USAGE: &str = "\
usage: ansa install-skill claude [--project]
           Install the Claude Code skill into ~/.claude/skills/ansa-bus
           (with --project: ./.claude/skills/ansa-bus).
       ansa install-skill codex
           Install the Codex CLI skill into ~/.codex/skills/ansa-bus
           (honours CODEX_HOME).
       ansa install-skill chatgpt [DIR]
           Write the Custom GPT instructions and OpenAPI spec into DIR
           (default ./ansa-chatgpt) and print setup steps.
";

/// Entry point for `ansa install-skill ...`. Returns the process exit code.
pub fn install(args: &[String]) -> i32 {
    let result = match args.first().map(String::as_str) {
        Some("claude") => install_claude(&args[1..]),
        Some("codex") => install_codex(&args[1..]),
        Some("chatgpt") => install_chatgpt(&args[1..]),
        _ => {
            eprint!("{USAGE}");
            return 2;
        }
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn install_claude(args: &[String]) -> io::Result<i32> {
    let base = match args.first().map(String::as_str) {
        None => match home_dir() {
            Some(home) => home.join(".claude").join("skills"),
            None => {
                eprintln!("cannot determine home directory (HOME/USERPROFILE unset)");
                return Ok(1);
            }
        },
        Some("--project") => PathBuf::from(".claude").join("skills"),
        Some(other) => {
            eprintln!("unknown argument: {other}");
            eprint!("{USAGE}");
            return Ok(2);
        }
    };
    write_file(&base.join("ansa-bus").join("SKILL.md"), CLAUDE_SKILL)?;
    println!("\nDone. In Claude Code: /ansa-bus, or ask the agent to \"join ansa\".");
    Ok(0)
}

fn install_codex(args: &[String]) -> io::Result<i32> {
    if let Some(other) = args.first() {
        eprintln!("unknown argument: {other}");
        eprint!("{USAGE}");
        return Ok(2);
    }
    // Codex only discovers skills under CODEX_HOME (default ~/.codex);
    // there is no per-project skills directory.
    let base = match codex_home() {
        Some(home) => home.join("skills"),
        None => {
            eprintln!("cannot determine home directory (CODEX_HOME/HOME/USERPROFILE unset)");
            return Ok(1);
        }
    };
    write_file(&base.join("ansa-bus").join("SKILL.md"), CODEX_SKILL)?;
    println!("\nDone. In Codex: /ansa-bus, or ask the agent to \"join ansa\".");
    Ok(0)
}

fn install_chatgpt(args: &[String]) -> io::Result<i32> {
    let dir = PathBuf::from(args.first().map_or("ansa-chatgpt", String::as_str));
    write_file(&dir.join("instructions.md"), CHATGPT_INSTRUCTIONS)?;
    write_file(&dir.join("ansa.openapi.yaml"), CHATGPT_OPENAPI)?;
    println!(
        "
ChatGPT cannot install these automatically. To finish:
  1. ChatGPT -> Explore GPTs -> Create -> Configure.
  2. Paste instructions.md into Instructions.
  3. Add an Action and import ansa.openapi.yaml.
  4. ChatGPT's servers cannot reach localhost: expose the hub at a
     public URL (e.g. `ngrok http 7777`) and put it in the OpenAPI
     `servers:` field before importing. ansa has no auth — only use
     a trusted, temporary tunnel."
    );
    Ok(0)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn codex_home() -> Option<PathBuf> {
    match std::env::var_os("CODEX_HOME") {
        Some(dir) => Some(PathBuf::from(dir)),
        None => home_dir().map(|home| home.join(".codex")),
    }
}

fn write_file(path: &Path, content: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    println!("wrote {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ansa-test-{}-{name}", std::process::id()))
    }

    #[test]
    fn chatgpt_files_land_in_target_dir() {
        let dir = tmp("chatgpt");
        let args = [dir.to_str().unwrap().to_string()];
        assert_eq!(install_chatgpt(&args).unwrap(), 0);
        let spec = std::fs::read_to_string(dir.join("ansa.openapi.yaml")).unwrap();
        assert!(spec.contains("openapi"));
        assert!(dir.join("instructions.md").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn claude_skill_has_frontmatter() {
        assert!(CLAUDE_SKILL.starts_with("---\nname: ansa-bus"));
    }

    #[test]
    fn codex_skill_has_frontmatter() {
        assert!(CODEX_SKILL.starts_with("---\nname: ansa-bus"));
    }

    #[test]
    fn codex_skill_lands_under_codex_home() {
        let home = tmp("codex-home");
        // CODEX_HOME is read by nothing else in the test binary.
        std::env::set_var("CODEX_HOME", &home);
        assert_eq!(install_codex(&[]).unwrap(), 0);
        let skill = std::fs::read_to_string(home.join("skills/ansa-bus/SKILL.md")).unwrap();
        assert!(skill.contains("name: ansa-bus"));
        std::env::remove_var("CODEX_HOME");
        std::fs::remove_dir_all(&home).unwrap();
    }
}
