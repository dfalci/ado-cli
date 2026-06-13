//! CLI (saída JSON) para tasks/work items de um board do Azure DevOps.
//!
//! Configuração via arquivo `.env` no diretório atual (fallback: ambiente do
//! SO). Veja `config.rs`. O subcomando `skill` instala a skill do Claude Code e
//! não exige configuração.

mod azure;
mod cli;
mod config;
mod ops;
mod skill;

use std::collections::HashMap;
use std::io::Read;

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde_json::Value;

use azure::AzureClient;
use cli::{Cli, Command};
use config::Config;
use ops::{ChildTask, CreateFields};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // O subcomando `skill` não toca na API nem exige configuração.
    if let Command::Skill = cli.command {
        return skill::install();
    }

    let config = Config::load()?;
    let client = AzureClient::new(config)?;
    let value = run(&client, cli.command).await?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

/// Converte um `Vec<String>` (possivelmente vazio) em `Option<&[String]>`,
/// onde vazio vira `None` (campos padrão).
fn opt_fields(fields: &[String]) -> Option<&[String]> {
    if fields.is_empty() {
        None
    } else {
        Some(fields)
    }
}

/// Lê todo o stdin como string.
fn read_stdin() -> Result<String> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .context("falha ao ler o stdin")?;
    Ok(buf)
}

/// Converte um objeto JSON em mapa `campo -> valor (string)`. Valores string são
/// usados como estão; demais tipos (número, bool) são convertidos para texto.
fn fields_from_json_object(value: Value) -> Result<HashMap<String, String>> {
    let obj = value
        .as_object()
        .context("o JSON de campos deve ser um objeto {\"ref\": valor}")?;
    let mut out = HashMap::new();
    for (k, v) in obj {
        let s = match v {
            Value::String(s) => s.clone(),
            Value::Null => continue,
            other => other.to_string(),
        };
        out.insert(k.clone(), s);
    }
    Ok(out)
}

/// Executa o comando contra o cliente e devolve o JSON de resultado.
async fn run(client: &AzureClient, command: Command) -> Result<Value> {
    Ok(match command {
        Command::Skill => unreachable!("tratado em main"),

        Command::Query {
            wiql,
            include_closed,
            fields,
        } => ops::query(client, wiql.as_deref(), include_closed, opt_fields(&fields)).await?,
        Command::Get { id } => ops::get(client, id).await?,
        Command::Links { id } => ops::links(client, id).await?,

        Command::Create {
            work_item_type,
            title,
            description,
            assigned_to,
            state,
            area_path,
            iteration_path,
            tags,
            priority,
            story_points,
            acceptance_criteria,
            repro_steps,
            original_estimate,
            remaining_work,
            parent_id,
        } => {
            let fields = CreateFields {
                work_item_type,
                title,
                description,
                assigned_to,
                state,
                area_path,
                iteration_path,
                tags: if tags.is_empty() { None } else { Some(tags) },
                priority,
                story_points,
                acceptance_criteria,
                repro_steps,
                original_estimate,
                remaining_work,
                parent_id,
            };
            ops::create(client, fields).await?
        }

        Command::Update { id, set, json } => {
            let mut fields = HashMap::new();
            // 1) --set ref=valor (repetível).
            for entry in &set {
                let (k, v) = entry
                    .split_once('=')
                    .with_context(|| format!("--set inválido (use ref=valor): {entry}"))?;
                fields.insert(k.trim().to_string(), v.to_string());
            }
            // 2) --json, ou stdin quando nem --set nem --json foram informados.
            let json_src = match json {
                Some(j) => Some(j),
                None if set.is_empty() => Some(read_stdin()?),
                None => None,
            };
            if let Some(src) = json_src {
                let parsed: Value =
                    serde_json::from_str(&src).context("JSON de campos inválido")?;
                fields.extend(fields_from_json_object(parsed)?);
            }
            if fields.is_empty() {
                bail!("nenhum campo informado: use --set ref=valor ou --json '{{...}}'");
            }
            ops::update(client, id, fields).await?
        }

        Command::SetState { id, state } => ops::set_state(client, id, &state).await?,
        Command::Assign { id, assigned_to } => ops::assign(client, id, &assigned_to).await?,

        Command::SetTaskboardColumn {
            id,
            column,
            iteration_id,
        } => ops::set_taskboard_column(client, id, &column, iteration_id).await?,

        Command::AddLink {
            id,
            target_id,
            link_type,
            comment,
        } => ops::add_link(client, id, target_id, link_type, comment).await?,

        Command::SetBacklogPriority {
            id,
            priority,
            field,
        } => ops::set_backlog_priority(client, id, priority, field.as_deref()).await?,

        Command::ListComments { id } => ops::list_comments(client, id).await?,
        Command::AddComment { id, text } => ops::add_comment(client, id, &text).await?,

        Command::ListIterations {
            timeframe,
            include_closed,
        } => ops::list_iterations(client, timeframe.as_deref(), include_closed).await?,

        Command::CurrentSprint { fields } => {
            ops::current_sprint(client, opt_fields(&fields)).await?
        }
        Command::MyWorkItems {
            only_current_sprint,
            include_closed,
            fields,
        } => {
            ops::my_work_items(client, only_current_sprint, include_closed, opt_fields(&fields))
                .await?
        }

        Command::TaskboardColumns => ops::taskboard_columns(client).await?,
        Command::Taskboard { fields } => ops::taskboard(client, opt_fields(&fields)).await?,

        Command::MoveToIteration { id, iteration_path } => {
            ops::move_to_iteration(client, id, &iteration_path).await?
        }
        Command::MoveToBacklog { id } => ops::move_to_backlog(client, id).await?,
        Command::MoveToCurrentSprint { id } => ops::move_to_current_sprint(client, id).await?,

        Command::CreateChildTasks { parent_id, json } => {
            let src = match json {
                Some(j) => j,
                None => read_stdin()?,
            };
            let tasks: Vec<ChildTask> =
                serde_json::from_str(&src).context("JSON de sub-tasks inválido (esperado array)")?;
            ops::create_child_tasks(client, parent_id, tasks).await?
        }

        Command::AddTags { id, tags } => ops::add_tags(client, id, tags).await?,
        Command::RemoveTags { id, tags } => ops::remove_tags(client, id, tags).await?,

        Command::ListWorkItemTypes => ops::list_work_item_types(client).await?,
        Command::ListTeamMembers => ops::list_team_members(client).await?,
        Command::SearchUsers { query } => ops::search_users(client, &query).await?,
    })
}
