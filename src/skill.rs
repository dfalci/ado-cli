//! Instalação da skill do Claude Code no projeto local.
//!
//! O subcomando `ado-cli skill` grava o `SKILL.md` (embutido no binário) em
//! `./.claude/skills/azure-devops-tasks/SKILL.md`, relativo ao diretório atual,
//! criando a árvore `.claude/...` se não existir e sobrescrevendo se existir.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Conteúdo da skill embutido no binário em tempo de compilação.
const SKILL_MD: &str = include_str!("../assets/SKILL.md");

/// Nome do diretório da skill.
const SKILL_NAME: &str = "azure-devops-tasks";

/// Instala a skill em `./.claude/skills/<nome>/SKILL.md`.
///
/// Sobrescreve um arquivo já existente (comportamento padrão).
pub fn install() -> Result<()> {
    let dir: PathBuf = [".claude", "skills", SKILL_NAME].iter().collect();
    let file = dir.join("SKILL.md");
    let existed = file.exists();

    std::fs::create_dir_all(&dir)
        .with_context(|| format!("não foi possível criar o diretório {}", dir.display()))?;
    std::fs::write(&file, SKILL_MD)
        .with_context(|| format!("não foi possível gravar {}", file.display()))?;

    if existed {
        println!("Skill sobrescrita em {}", file.display());
    } else {
        println!("Skill instalada em {}", file.display());
    }
    Ok(())
}
