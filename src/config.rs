//! Carregamento e validação da configuração da CLI.
//!
//! A configuração vem de um arquivo `.env` (formato `CHAVE=valor`) no diretório
//! atual. Para cada chave ausente no arquivo, cai-se para a variável de ambiente
//! do SO de mesmo nome. Não há flags de linha de comando para configuração — a
//! linha de comando recebe apenas os argumentos das operações.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Context, Result};

/// Versão default da API REST do Azure DevOps.
const DEFAULT_API_VERSION: &str = "7.1";
/// URL base default (Azure DevOps Services na nuvem).
const DEFAULT_BASE_URL: &str = "https://dev.azure.com";
/// Nome do arquivo de environment lido do diretório atual.
const ENV_FILE: &str = ".env";

/// Configuração resolvida e validada da CLI.
#[derive(Debug, Clone)]
pub struct Config {
    /// Personal Access Token usado na autenticação Basic.
    pub pat: String,
    /// Organização do Azure DevOps (extraída de `org/projeto`).
    pub organization: String,
    /// Projeto dentro da organização.
    pub project: String,
    /// Time (team) usado nas APIs de iteração/sprint. Default: `{project} Team`.
    pub team: String,
    /// URL base (ex.: `https://dev.azure.com` ou um host on-prem).
    pub base_url: String,
    /// Versão da API REST (ex.: `7.1`).
    pub api_version: String,
}

impl Config {
    /// Carrega a configuração lendo o `.env` do diretório atual e usando as
    /// variáveis de ambiente do SO como fallback por chave.
    pub fn load() -> Result<Self> {
        let file_vars = load_env_file(Path::new(ENV_FILE))?;
        Self::from_lookup(|k| file_vars.get(k).cloned().or_else(|| std::env::var(k).ok()))
    }

    /// Implementação testável: recebe uma função de lookup que resolve cada chave
    /// (arquivo `.env` primeiro, ambiente do SO depois).
    pub fn from_lookup<F>(lookup: F) -> Result<Self>
    where
        F: Fn(&str) -> Option<String>,
    {
        let get = |key: &str| lookup(key).filter(|s| !s.trim().is_empty());

        let pat = get("AZDO_PAT")
            .context("PAT não informado: defina AZDO_PAT no arquivo .env (ou no ambiente)")?;

        let project_spec = get("AZDO_PROJECT").context(
            "Projeto não informado: defina AZDO_PROJECT (org/projeto) no arquivo .env (ou no ambiente)",
        )?;

        let (organization, project) = split_project_spec(&project_spec)?;

        let team = get("AZDO_TEAM").unwrap_or_else(|| format!("{project} Team"));
        let base_url = get("AZDO_BASE_URL").unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let api_version =
            get("AZDO_API_VERSION").unwrap_or_else(|| DEFAULT_API_VERSION.to_string());

        Ok(Config {
            pat,
            organization,
            project,
            team,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_version,
        })
    }
}

/// Lê um arquivo `.env` simples (`CHAVE=valor` por linha). Ignora linhas vazias e
/// comentários (`#`), aceita o prefixo `export `, e remove aspas simples/duplas
/// que envolvam o valor. Arquivo inexistente não é erro (retorna mapa vazio).
fn load_env_file(path: &Path) -> Result<HashMap<String, String>> {
    let mut vars = HashMap::new();
    if !path.exists() {
        return Ok(vars);
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("não foi possível ler {}", path.display()))?;
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = unquote(value.trim());
        vars.insert(key.to_string(), value);
    }
    Ok(vars)
}

/// Remove aspas simples ou duplas que envolvam totalmente o valor.
fn unquote(s: &str) -> String {
    let bytes = s.as_bytes();
    if s.len() >= 2 {
        let first = bytes[0];
        let last = bytes[s.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return s[1..s.len() - 1].to_string();
        }
    }
    s.to_string()
}

/// Separa a especificação `org/projeto` em organização e projeto.
fn split_project_spec(spec: &str) -> Result<(String, String)> {
    let spec = spec.trim().trim_matches('/');
    let (org, project) = spec
        .split_once('/')
        .context("AZDO_PROJECT deve estar no formato 'organizacao/projeto'")?;
    let org = org.trim();
    let project = project.trim();
    if org.is_empty() || project.is_empty() {
        bail!("AZDO_PROJECT deve estar no formato 'organizacao/projeto' (ambos não vazios)");
    }
    // O projeto pode conter espaços, mas não uma segunda barra.
    if project.contains('/') {
        bail!("AZDO_PROJECT deve conter exatamente uma barra separando organização e projeto");
    }
    Ok((org.to_string(), project.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup_from(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k: &str| map.get(k).cloned()
    }

    #[test]
    fn split_org_e_projeto() {
        let (org, proj) = split_project_spec("myorg/MyProject").unwrap();
        assert_eq!(org, "myorg");
        assert_eq!(proj, "MyProject");
    }

    #[test]
    fn split_projeto_com_espaco() {
        let (org, proj) = split_project_spec("myorg/My Project").unwrap();
        assert_eq!(org, "myorg");
        assert_eq!(proj, "My Project");
    }

    #[test]
    fn split_invalido_sem_barra() {
        assert!(split_project_spec("sembarra").is_err());
    }

    #[test]
    fn config_a_partir_do_lookup() {
        let cfg = Config::from_lookup(lookup_from(&[
            ("AZDO_PAT", "token"),
            ("AZDO_PROJECT", "envorg/EnvProj"),
        ]))
        .unwrap();
        assert_eq!(cfg.pat, "token");
        assert_eq!(cfg.organization, "envorg");
        assert_eq!(cfg.project, "EnvProj");
        assert_eq!(cfg.api_version, "7.1");
        assert_eq!(cfg.base_url, "https://dev.azure.com");
        assert_eq!(cfg.team, "EnvProj Team");
    }

    #[test]
    fn falta_pat_falha() {
        let res = Config::from_lookup(lookup_from(&[("AZDO_PROJECT", "org/Proj")]));
        assert!(res.is_err());
    }

    #[test]
    fn team_override_e_trim_base_url() {
        let cfg = Config::from_lookup(lookup_from(&[
            ("AZDO_PAT", "t"),
            ("AZDO_PROJECT", "org/Loja"),
            ("AZDO_TEAM", "Squad A"),
            ("AZDO_BASE_URL", "https://server/tfs/"),
        ]))
        .unwrap();
        assert_eq!(cfg.team, "Squad A");
        assert_eq!(cfg.base_url, "https://server/tfs");
    }

    #[test]
    fn valor_em_branco_e_ignorado() {
        // PAT em branco no "arquivo" cai para a chave de ambiente (aqui, ausente).
        let res = Config::from_lookup(lookup_from(&[
            ("AZDO_PAT", "   "),
            ("AZDO_PROJECT", "org/Proj"),
        ]));
        assert!(res.is_err());
    }

    #[test]
    fn unquote_remove_aspas() {
        assert_eq!(unquote("\"abc\""), "abc");
        assert_eq!(unquote("'abc'"), "abc");
        assert_eq!(unquote("abc"), "abc");
        assert_eq!(unquote("\"a"), "\"a");
    }
}
