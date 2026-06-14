//! Instalação da skill do Claude Code no projeto local.
//!
//! O subcomando `ado-cli skill` grava o `SKILL.md` (embutido no binário) em
//! `./.claude/skills/azure-devops-tasks/SKILL.md` e, num terminal interativo,
//! pergunta as credenciais e grava o `.env` na MESMA pasta — que é onde a CLI
//! procura a configuração (ver `config.rs`). Também grava um `.env.example`.

use anyhow::{bail, Context, Result};
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

/// Conteúdo da skill embutido no binário em tempo de compilação.
const SKILL_MD: &str = include_str!("../assets/SKILL.md");

/// Modelo de `.env` embutido, gravado na pasta da skill na instalação.
const ENV_EXAMPLE: &str = include_str!("../assets/env.example");

/// Nome do diretório da skill.
pub const SKILL_NAME: &str = "azure-devops-tasks";

/// Diretório da skill, relativo ao diretório atual: `.claude/skills/<nome>`.
pub fn skill_dir() -> PathBuf {
    [".claude", "skills", SKILL_NAME].iter().collect()
}

/// Caminho do arquivo `.env` lido pela CLI: dentro da pasta da skill.
pub fn env_path() -> PathBuf {
    skill_dir().join(".env")
}

/// Instala a skill (sobrescreve o `SKILL.md`), grava um `.env.example` e, num
/// terminal interativo, configura o `.env` perguntando as credenciais.
pub fn install() -> Result<()> {
    let dir = skill_dir();
    let file = dir.join("SKILL.md");
    let existed = file.exists();

    std::fs::create_dir_all(&dir)
        .with_context(|| format!("não foi possível criar o diretório {}", dir.display()))?;
    std::fs::write(&file, SKILL_MD)
        .with_context(|| format!("não foi possível gravar {}", file.display()))?;

    let example = dir.join(".env.example");
    if !example.exists() {
        std::fs::write(&example, ENV_EXAMPLE)
            .with_context(|| format!("não foi possível gravar {}", example.display()))?;
    }

    if existed {
        println!("Skill sobrescrita em {}", file.display());
    } else {
        println!("Skill instalada em {}", file.display());
    }

    configure_env(&dir)?;
    Ok(())
}

/// Configura o `.env` da pasta da skill. Num terminal interativo, pergunta as
/// credenciais e grava o arquivo; caso contrário, apenas instrui onde preenchê-lo.
fn configure_env(dir: &Path) -> Result<()> {
    let env_file = dir.join(".env");

    if !io::stdin().is_terminal() {
        println!(
            "Configure as credenciais em {} (modelo em {}).",
            env_file.display(),
            dir.join(".env.example").display()
        );
        return Ok(());
    }

    if env_file.exists()
        && !ask_yes_no(
            &format!("Já existe {}. Sobrescrever?", env_file.display()),
            false,
        )?
    {
        println!("Mantido o .env existente. Skill pronta para uso.");
        return Ok(());
    }

    println!(
        "\nConfiguração do acesso ao Azure DevOps (será gravada em {}):",
        env_file.display()
    );
    let pat = prompt_required("Personal Access Token (AZDO_PAT)")?;
    let project = prompt_project()?;
    let team = prompt_optional("Time (AZDO_TEAM)", "{projeto} Team")?;
    let base_url = prompt_optional("Base URL (AZDO_BASE_URL)", "https://dev.azure.com")?;
    let api_version = prompt_optional("Versão da API (AZDO_API_VERSION)", "7.1")?;

    let mut content = format!("AZDO_PAT={pat}\nAZDO_PROJECT={project}\n");
    if let Some(t) = team {
        content.push_str(&format!("AZDO_TEAM={t}\n"));
    }
    if let Some(b) = base_url {
        content.push_str(&format!("AZDO_BASE_URL={b}\n"));
    }
    if let Some(a) = api_version {
        content.push_str(&format!("AZDO_API_VERSION={a}\n"));
    }
    std::fs::write(&env_file, content)
        .with_context(|| format!("não foi possível gravar {}", env_file.display()))?;
    println!(
        "Configuração salva em {}. Skill pronta para uso.",
        env_file.display()
    );
    Ok(())
}

/// Lê uma linha do stdin (trim). Falha em EOF.
fn read_line() -> Result<String> {
    let mut s = String::new();
    let n = io::stdin()
        .read_line(&mut s)
        .context("falha ao ler a entrada")?;
    if n == 0 {
        bail!("entrada encerrada");
    }
    Ok(s.trim().to_string())
}

/// Pergunta um valor obrigatório (repete enquanto vazio).
fn prompt_required(label: &str) -> Result<String> {
    loop {
        print!("  {label}: ");
        io::stdout().flush().ok();
        let v = read_line()?;
        if !v.is_empty() {
            return Ok(v);
        }
        println!("  (obrigatório)");
    }
}

/// Pergunta um valor opcional; vazio = usar o default (retorna None).
fn prompt_optional(label: &str, default_hint: &str) -> Result<Option<String>> {
    print!("  {label} [{default_hint}]: ");
    io::stdout().flush().ok();
    let v = read_line()?;
    Ok(if v.is_empty() { None } else { Some(v) })
}

/// Pergunta o projeto validando o formato `organizacao/projeto`.
fn prompt_project() -> Result<String> {
    loop {
        let v = prompt_required("Projeto (organizacao/projeto) (AZDO_PROJECT)")?;
        let t = v.trim().trim_matches('/');
        if let Some((org, proj)) = t.split_once('/') {
            if !org.trim().is_empty() && !proj.trim().is_empty() && !proj.contains('/') {
                return Ok(t.to_string());
            }
        }
        println!("  formato inválido — use organizacao/projeto (ex.: contoso/Loja)");
    }
}

/// Pergunta sim/não, com default configurável.
fn ask_yes_no(question: &str, default_yes: bool) -> Result<bool> {
    let hint = if default_yes { "[S/n]" } else { "[s/N]" };
    print!("{question} {hint} ");
    io::stdout().flush().ok();
    let v = read_line()?.to_lowercase();
    if v.is_empty() {
        return Ok(default_yes);
    }
    Ok(matches!(v.as_str(), "s" | "sim" | "y" | "yes"))
}
