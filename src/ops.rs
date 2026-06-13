//! Operações de alto nível sobre o board: combinam chamadas do `AzureClient`,
//! montam os JSON Patches e compõem as respostas (árvore de links, visão do
//! taskboard, etc.). Cada função retorna um `serde_json::Value` pronto para ser
//! impresso como JSON pela CLI.

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::azure::{
    models::{Iteration, JsonPatchOp, WorkItem},
    AzureClient,
};

/// Campos de exibição enxutos usados nos nós das árvores/listas de relação.
const DISPLAY_FIELDS: &[&str] = &[
    "System.Id",
    "System.Title",
    "System.WorkItemType",
    "System.State",
    "System.AssignedTo",
];

/// Tipo de vínculo entre dois work items (espelha as opções da linha de comando).
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum LinkType {
    /// O alvo é pai do work item (Hierarchy-Reverse).
    Parent,
    /// O alvo é filho do work item (Hierarchy-Forward).
    Child,
    /// Relação genérica (Related).
    Related,
    /// O alvo é predecessor (Dependency-Reverse).
    Predecessor,
    /// O alvo é sucessor (Dependency-Forward).
    Successor,
}

impl LinkType {
    /// Reference name do tipo de link na API.
    pub fn rel(&self) -> &'static str {
        match self {
            LinkType::Parent => "System.LinkTypes.Hierarchy-Reverse",
            LinkType::Child => "System.LinkTypes.Hierarchy-Forward",
            LinkType::Related => "System.LinkTypes.Related",
            LinkType::Predecessor => "System.LinkTypes.Dependency-Reverse",
            LinkType::Successor => "System.LinkTypes.Dependency-Forward",
        }
    }
}

/// Campos para criar um work item (espelha os parâmetros do subcomando `create`).
#[derive(Debug, Default)]
pub struct CreateFields {
    pub work_item_type: String,
    pub title: String,
    pub description: Option<String>,
    pub assigned_to: Option<String>,
    pub state: Option<String>,
    pub area_path: Option<String>,
    pub iteration_path: Option<String>,
    pub tags: Option<Vec<String>>,
    pub priority: Option<i64>,
    pub story_points: Option<f64>,
    pub acceptance_criteria: Option<String>,
    pub repro_steps: Option<String>,
    pub original_estimate: Option<f64>,
    pub remaining_work: Option<f64>,
    pub parent_id: Option<i64>,
}

/// Uma sub-task a criar sob um pai em `create_child_tasks` (parse de JSON).
#[derive(Debug, Deserialize)]
pub struct ChildTask {
    pub title: String,
    #[serde(default)]
    pub work_item_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub assigned_to: Option<String>,
    #[serde(default)]
    pub remaining_work: Option<f64>,
}

// ---- Leitura --------------------------------------------------------------

pub async fn query(
    client: &AzureClient,
    wiql: Option<&str>,
    include_closed: bool,
    fields: Option<&[String]>,
) -> Result<Value> {
    let items = match wiql {
        // WIQL explícita: respeitada como está — não injetamos filtros.
        Some(w) => client.query_work_items(Some(w), fields).await?,
        // Sem WIQL (busca exploratória default): por padrão, só itens ABERTOS.
        None => {
            let mut q =
                String::from("SELECT [System.Id] FROM WorkItems WHERE [System.TeamProject] = @project");
            q.push_str(&open_filter_clause(client, include_closed).await?);
            q.push_str(" ORDER BY [System.ChangedDate] DESC");
            client.query_work_items(Some(&q), fields).await?
        }
    };
    Ok(json!(items))
}

pub async fn get(client: &AzureClient, id: i64) -> Result<Value> {
    Ok(json!(client.get_work_item(id).await?))
}

pub async fn list_comments(client: &AzureClient, id: i64) -> Result<Value> {
    let comments = client.list_comments(id).await?;
    Ok(json!({ "count": comments.count, "comments": comments.comments }))
}

pub async fn list_work_item_types(client: &AzureClient) -> Result<Value> {
    Ok(json!(client.list_work_item_types().await?))
}

pub async fn list_team_members(client: &AzureClient) -> Result<Value> {
    Ok(json!(client.list_team_members().await?))
}

pub async fn search_users(client: &AzureClient, query: &str) -> Result<Value> {
    Ok(json!(client.search_users(query).await?))
}

pub async fn taskboard_columns(client: &AzureClient) -> Result<Value> {
    Ok(json!(client.get_taskboard_columns().await?))
}

pub async fn list_iterations(
    client: &AzureClient,
    timeframe: Option<&str>,
    include_closed: bool,
) -> Result<Value> {
    // Busca todas e filtra client-side por attributes.timeFrame: a API só
    // filtra "current" no servidor, então past/future e "abertas" são nossos.
    let all = client.list_iterations(None).await?;
    let filtered: Vec<Iteration> = match timeframe {
        Some(tf) => {
            let tf = tf.trim().to_lowercase();
            all.into_iter()
                .filter(|it| iteration_timeframe(it) == tf)
                .collect()
        }
        None if include_closed => all,
        // Padrão: tudo que não é "past" (inclui sprints sem datas definidas).
        None => all
            .into_iter()
            .filter(|it| iteration_timeframe(it) != "past")
            .collect(),
    };
    Ok(json!(filtered))
}

pub async fn current_sprint(client: &AzureClient, fields: Option<&[String]>) -> Result<Value> {
    let iteration = client.current_iteration().await?;
    let Some(iteration) = iteration else {
        return Ok(json!({ "iteration": null, "count": 0, "work_item_ids": [] }));
    };
    let ids = client.iteration_work_item_ids(&iteration.id).await?;
    match fields.filter(|f| !f.is_empty()) {
        Some(fields) => {
            let work_items = if ids.is_empty() {
                Vec::new()
            } else {
                client.get_work_items(&ids, Some(fields)).await?
            };
            Ok(json!({ "iteration": iteration, "work_items": work_items }))
        }
        None => Ok(json!({ "iteration": iteration, "count": ids.len(), "work_item_ids": ids })),
    }
}

pub async fn my_work_items(
    client: &AzureClient,
    only_current_sprint: bool,
    include_closed: bool,
    fields: Option<&[String]>,
) -> Result<Value> {
    let mut wiql = String::from(
        "SELECT [System.Id] FROM WorkItems \
         WHERE [System.TeamProject] = @project AND [System.AssignedTo] = @Me",
    );
    // Por padrão, só itens ABERTOS (exclui os estados terminais).
    wiql.push_str(&open_filter_clause(client, include_closed).await?);
    if only_current_sprint {
        if let Some(it) = client.current_iteration().await? {
            let path = it.path.replace('\'', "''");
            wiql.push_str(&format!(" AND [System.IterationPath] UNDER '{path}'"));
        }
    }
    wiql.push_str(" ORDER BY [System.ChangedDate] DESC");
    let items = client.query_work_items(Some(&wiql), fields).await?;
    Ok(json!(items))
}

/// Cláusula WIQL `" AND [System.State] NOT IN (...)"` que exclui os itens
/// fechados, ou string vazia quando `include_closed` ou quando o projeto não
/// expõe estados terminais. Compartilhada por `query` e `my_work_items`.
async fn open_filter_clause(client: &AzureClient, include_closed: bool) -> Result<String> {
    if include_closed {
        return Ok(String::new());
    }
    let closed = closed_states(client).await?;
    if closed.is_empty() {
        return Ok(String::new());
    }
    let list = closed
        .iter()
        .map(|s| format!("'{}'", s.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(format!(" AND [System.State] NOT IN ({list})"))
}

/// Nomes dos estados considerados "fechados" no projeto: aqueles cuja categoria
/// é terminal ("Completed" ou "Removed"). Descobertos via `list-work-item-types`
/// para não depender de nomes fixos (Closed/Done/Resolved variam por processo).
async fn closed_states(client: &AzureClient) -> Result<Vec<String>> {
    let mut set = std::collections::BTreeSet::new();
    for t in client.list_work_item_types().await? {
        for st in t.states {
            if matches!(st.category.as_deref(), Some("Completed" | "Removed")) && !st.name.is_empty()
            {
                set.insert(st.name);
            }
        }
    }
    Ok(set.into_iter().collect())
}

pub async fn links(client: &AzureClient, id: i64) -> Result<Value> {
    // Limite de segurança contra árvores enormes/ciclos.
    const MAX_NODES: usize = 400;

    // 1. Raiz (com relações).
    let root = client
        .get_work_items_with_relations(&[id])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("work item {id} não encontrado"))?;
    let (root_children, root_parent, root_related, root_pred, root_succ) = classify_relations(&root);

    // 2. Desce a hierarquia de filhos por nível (BFS), guardando relações.
    let mut nodes: HashMap<i64, WorkItem> = HashMap::new();
    let mut children_of: HashMap<i64, Vec<i64>> = HashMap::new();
    children_of.insert(id, root_children.clone());
    nodes.insert(id, root.clone());

    let mut frontier = root_children.clone();
    let mut truncated = false;
    while !frontier.is_empty() {
        let batch: Vec<i64> = frontier
            .iter()
            .copied()
            .filter(|id| !nodes.contains_key(id))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        if batch.is_empty() {
            break;
        }
        if nodes.len() + batch.len() > MAX_NODES {
            truncated = true;
            break;
        }
        let mut next = Vec::new();
        for chunk in batch.chunks(200) {
            let items = client.get_work_items_with_relations(chunk).await?;
            for it in items {
                let (ch, _, _, _, _) = classify_relations(&it);
                next.extend(ch.iter().copied());
                children_of.insert(it.id, ch);
                nodes.insert(it.id, it);
            }
        }
        frontier = next;
    }

    // 3. Cadeia de pais (ascendente).
    let mut parents = Vec::new();
    let mut current_parent = root_parent;
    let mut guard = 0;
    while let Some(pid) = current_parent {
        guard += 1;
        if guard > 50 {
            break;
        }
        let fetched = client.get_work_items_with_relations(&[pid]).await?;
        let Some(p) = fetched.into_iter().next() else {
            break;
        };
        let (_, pp, _, _, _) = classify_relations(&p);
        parents.push(display_node(&p));
        current_parent = pp;
    }

    // 4. related/predecessores/sucessores diretos (hidratados enxutos).
    let related = hydrate_display(client, &root_related).await?;
    let predecessors = hydrate_display(client, &root_pred).await?;
    let successors = hydrate_display(client, &root_succ).await?;

    // 5. Monta a árvore de filhos a partir dos mapas em memória.
    let mut visited = HashSet::new();
    let children: Vec<Value> = root_children
        .iter()
        .map(|c| build_subtree(*c, &nodes, &children_of, &mut visited))
        .collect();

    Ok(json!({
        "item": display_node(&root),
        "parents": parents,
        "children": children,
        "related": related,
        "predecessors": predecessors,
        "successors": successors,
        "truncated": truncated,
    }))
}

pub async fn taskboard(client: &AzureClient, fields: Option<&[String]>) -> Result<Value> {
    let iteration = client.current_iteration().await?;
    let Some(iteration) = iteration else {
        return Ok(json!({ "iteration": null, "columns": [] }));
    };

    // Config das colunas (ordem + nomes) e a posição de cada item no taskboard.
    let cols = client.get_taskboard_columns().await?;
    let positions = client.taskboard_work_items(&iteration.id).await?;

    // Hidrata os work items que aparecem no taskboard.
    let ids: Vec<i64> = positions.iter().map(|p| p.work_item_id).collect();
    let default_fields: Vec<String> = DISPLAY_FIELDS.iter().map(|s| s.to_string()).collect();
    let fields: &[String] = match fields.filter(|f| !f.is_empty()) {
        Some(f) => f,
        None => &default_fields,
    };
    let items = if ids.is_empty() {
        Vec::new()
    } else {
        client.get_work_items(&ids, Some(fields)).await?
    };
    let item_by_id: HashMap<i64, WorkItem> = items.into_iter().map(|w| (w.id, w)).collect();

    // Coluna (nome) de cada work item, segundo o taskboard.
    let column_of: HashMap<i64, String> = positions
        .iter()
        .filter_map(|p| p.column.clone().map(|c| (p.work_item_id, c)))
        .collect();

    // Monta os grupos na ordem das colunas configuradas (inclui colunas vazias).
    let configured: HashSet<&str> = cols.columns.iter().map(|c| c.name.as_str()).collect();
    let mut columns_out: Vec<Value> = Vec::new();
    for col in &cols.columns {
        let col_items: Vec<&WorkItem> = positions
            .iter()
            .filter(|p| column_of.get(&p.work_item_id).map(String::as_str) == Some(col.name.as_str()))
            .filter_map(|p| item_by_id.get(&p.work_item_id))
            .collect();
        columns_out.push(json!({
            "column": col.name,
            "columnId": col.id,
            "order": col.order,
            "count": col_items.len(),
            "items": col_items,
        }));
    }

    // Itens sem coluna mapeada (raro) vão para um grupo extra, para não sumirem.
    let leftover: Vec<&WorkItem> = positions
        .iter()
        .filter(|p| {
            column_of
                .get(&p.work_item_id)
                .map(|c| !configured.contains(c.as_str()))
                .unwrap_or(true)
        })
        .filter_map(|p| item_by_id.get(&p.work_item_id))
        .collect();
    if !leftover.is_empty() {
        columns_out.push(json!({
            "column": "(sem coluna)",
            "count": leftover.len(),
            "items": leftover,
        }));
    }

    Ok(json!({
        "iteration": iteration,
        "is_customized": cols.is_customized,
        "columns": columns_out,
    }))
}

// ---- Escrita --------------------------------------------------------------

pub async fn create(client: &AzureClient, f: CreateFields) -> Result<Value> {
    let mut ops = vec![JsonPatchOp::add_field("System.Title", f.title)];
    if let Some(d) = f.description {
        ops.push(JsonPatchOp::add_field("System.Description", d));
    }
    if let Some(a) = f.assigned_to {
        ops.push(JsonPatchOp::add_field("System.AssignedTo", a));
    }
    if let Some(s) = f.state {
        ops.push(JsonPatchOp::add_field("System.State", s));
    }
    if let Some(a) = f.area_path {
        ops.push(JsonPatchOp::add_field("System.AreaPath", a));
    }
    if let Some(i) = f.iteration_path {
        ops.push(JsonPatchOp::add_field("System.IterationPath", i));
    }
    if let Some(t) = f.tags {
        ops.push(JsonPatchOp::add_field("System.Tags", t.join("; ")));
    }
    if let Some(p) = f.priority {
        ops.push(JsonPatchOp::add_field("Microsoft.VSTS.Common.Priority", p));
    }
    if let Some(sp) = f.story_points {
        ops.push(JsonPatchOp::add_field("Microsoft.VSTS.Scheduling.StoryPoints", sp));
    }
    if let Some(ac) = f.acceptance_criteria {
        ops.push(JsonPatchOp::add_field("Microsoft.VSTS.Common.AcceptanceCriteria", ac));
    }
    if let Some(rs) = f.repro_steps {
        ops.push(JsonPatchOp::add_field("Microsoft.VSTS.TCM.ReproSteps", rs));
    }
    if let Some(oe) = f.original_estimate {
        ops.push(JsonPatchOp::add_field("Microsoft.VSTS.Scheduling.OriginalEstimate", oe));
    }
    if let Some(rw) = f.remaining_work {
        ops.push(JsonPatchOp::add_field("Microsoft.VSTS.Scheduling.RemainingWork", rw));
    }
    if let Some(parent_id) = f.parent_id {
        ops.push(JsonPatchOp::add_relation(
            LinkType::Parent.rel(),
            client.work_item_api_url(parent_id),
            None,
        ));
    }
    let item = client.create_work_item(&f.work_item_type, &ops).await?;
    Ok(json!(item))
}

pub async fn update(client: &AzureClient, id: i64, fields: HashMap<String, String>) -> Result<Value> {
    let ops: Vec<JsonPatchOp> = fields
        .into_iter()
        .map(|(k, v)| JsonPatchOp::add_field(&k, v))
        .collect();
    let item = client.update_work_item(id, &ops).await?;
    Ok(json!(item))
}

pub async fn set_state(client: &AzureClient, id: i64, state: &str) -> Result<Value> {
    let ops = vec![JsonPatchOp::add_field("System.State", state)];
    Ok(json!(client.update_work_item(id, &ops).await?))
}

pub async fn assign(client: &AzureClient, id: i64, assigned_to: &str) -> Result<Value> {
    let ops = vec![JsonPatchOp::add_field("System.AssignedTo", assigned_to)];
    Ok(json!(client.update_work_item(id, &ops).await?))
}

pub async fn add_link(
    client: &AzureClient,
    id: i64,
    target_id: i64,
    link_type: LinkType,
    comment: Option<String>,
) -> Result<Value> {
    let item = client
        .add_link(id, target_id, link_type.rel(), comment)
        .await?;
    Ok(json!(item))
}

pub async fn set_backlog_priority(
    client: &AzureClient,
    id: i64,
    priority: f64,
    field: Option<&str>,
) -> Result<Value> {
    let field = field.unwrap_or("Microsoft.VSTS.Common.StackRank");
    let ops = vec![JsonPatchOp::add_field(field, priority)];
    Ok(json!(client.update_work_item(id, &ops).await?))
}

pub async fn add_comment(client: &AzureClient, id: i64, text: &str) -> Result<Value> {
    Ok(json!(client.add_comment(id, text).await?))
}

pub async fn move_to_iteration(client: &AzureClient, id: i64, path: &str) -> Result<Value> {
    Ok(json!(client.set_iteration_path(id, path).await?))
}

pub async fn move_to_backlog(client: &AzureClient, id: i64) -> Result<Value> {
    let path = client.backlog_iteration_path().await?;
    Ok(json!(client.set_iteration_path(id, &path).await?))
}

pub async fn move_to_current_sprint(client: &AzureClient, id: i64) -> Result<Value> {
    let iteration = client
        .current_iteration()
        .await?
        .ok_or_else(|| anyhow!("nenhuma sprint atual configurada para o time"))?;
    Ok(json!(client.set_iteration_path(id, &iteration.path).await?))
}

pub async fn set_taskboard_column(
    client: &AzureClient,
    id: i64,
    column: &str,
    iteration_id: Option<String>,
) -> Result<Value> {
    let iteration_id = match iteration_id {
        Some(id) => id,
        None => {
            client
                .current_iteration()
                .await?
                .ok_or_else(|| anyhow!("nenhuma sprint atual configurada para o time"))?
                .id
        }
    };
    client.set_taskboard_column(&iteration_id, id, column).await?;
    Ok(json!({
        "id": id,
        "iteration_id": iteration_id,
        "column": column,
        "moved": true,
    }))
}

pub async fn create_child_tasks(
    client: &AzureClient,
    parent_id: i64,
    tasks: Vec<ChildTask>,
) -> Result<Value> {
    let parent_url = client.work_item_api_url(parent_id);
    let mut created = Vec::with_capacity(tasks.len());
    for task in tasks {
        let wtype = task.work_item_type.as_deref().unwrap_or("Task");
        let mut ops = vec![JsonPatchOp::add_field("System.Title", task.title)];
        if let Some(d) = task.description {
            ops.push(JsonPatchOp::add_field("System.Description", d));
        }
        if let Some(a) = task.assigned_to {
            ops.push(JsonPatchOp::add_field("System.AssignedTo", a));
        }
        if let Some(rw) = task.remaining_work {
            ops.push(JsonPatchOp::add_field("Microsoft.VSTS.Scheduling.RemainingWork", rw));
        }
        ops.push(JsonPatchOp::add_relation(
            LinkType::Parent.rel(),
            parent_url.clone(),
            None,
        ));
        created.push(client.create_work_item(wtype, &ops).await?);
    }
    Ok(json!({ "parent_id": parent_id, "created": created }))
}

pub async fn add_tags(client: &AzureClient, id: i64, tags: Vec<String>) -> Result<Value> {
    let mut current = client.get_tags(id).await?;
    for t in tags {
        let t = t.trim().to_string();
        if !t.is_empty() && !current.iter().any(|x| x.eq_ignore_ascii_case(&t)) {
            current.push(t);
        }
    }
    Ok(json!(client.set_tags(id, &current).await?))
}

pub async fn remove_tags(client: &AzureClient, id: i64, tags: Vec<String>) -> Result<Value> {
    let mut current = client.get_tags(id).await?;
    current.retain(|x| !tags.iter().any(|t| t.trim().eq_ignore_ascii_case(x)));
    Ok(json!(client.set_tags(id, &current).await?))
}

// ---- Helpers --------------------------------------------------------------

/// Hidrata ids em objetos de exibição enxutos (id, título, tipo, estado, resp.).
async fn hydrate_display(client: &AzureClient, ids: &[i64]) -> Result<Vec<Value>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let fields: Vec<String> = DISPLAY_FIELDS.iter().map(|s| s.to_string()).collect();
    let items = client.get_work_items(ids, Some(&fields)).await?;
    Ok(items.iter().map(display_node).collect())
}

/// Extrai o id do work item alvo do final de uma url de relação.
fn rel_target_id(url: &str) -> Option<i64> {
    url.rsplit('/').next().and_then(|s| s.parse::<i64>().ok())
}

/// Relações de um work item classificadas por tipo:
/// (filhos, pai, related, predecessores, sucessores).
type ClassifiedRelations = (Vec<i64>, Option<i64>, Vec<i64>, Vec<i64>, Vec<i64>);

/// Classifica as relações de um work item por tipo, retornando os ids alvo.
fn classify_relations(item: &WorkItem) -> ClassifiedRelations {
    let (mut children, mut related, mut predecessors, mut successors) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut parent = None;
    let rels = item.relations.as_ref().and_then(|r| r.as_array());
    for rel in rels.into_iter().flatten() {
        let Some(name) = rel.get("rel").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(id) = rel.get("url").and_then(|v| v.as_str()).and_then(rel_target_id) else {
            continue;
        };
        match name {
            "System.LinkTypes.Hierarchy-Forward" => children.push(id),
            "System.LinkTypes.Hierarchy-Reverse" => parent = Some(id),
            "System.LinkTypes.Related" => related.push(id),
            "System.LinkTypes.Dependency-Reverse" => predecessors.push(id),
            "System.LinkTypes.Dependency-Forward" => successors.push(id),
            _ => {}
        }
    }
    (children, parent, related, predecessors, successors)
}

/// Monta o objeto de exibição enxuto de um work item.
fn display_node(item: &WorkItem) -> Value {
    let f = &item.fields;
    json!({
        "id": item.id,
        "title": f.get("System.Title"),
        "type": f.get("System.WorkItemType"),
        "state": f.get("System.State"),
        "assignedTo": f.get("System.AssignedTo"),
    })
}

/// Constrói recursivamente a subárvore de filhos a partir dos mapas em memória.
fn build_subtree(
    id: i64,
    nodes: &HashMap<i64, WorkItem>,
    children_of: &HashMap<i64, Vec<i64>>,
    visited: &mut HashSet<i64>,
) -> Value {
    let mut node = nodes
        .get(&id)
        .map(display_node)
        .unwrap_or_else(|| json!({ "id": id }));
    if visited.insert(id) {
        let kids: Vec<Value> = children_of
            .get(&id)
            .into_iter()
            .flatten()
            .map(|c| build_subtree(*c, nodes, children_of, visited))
            .collect();
        if !kids.is_empty() {
            node["children"] = json!(kids);
        }
    }
    node
}

/// Lê o `timeFrame` ("past"/"current"/"future") de uma iteração, em minúsculas.
/// Retorna "" quando a sprint não tem datas/atributos (tratada como aberta).
fn iteration_timeframe(it: &Iteration) -> String {
    it.attributes
        .as_ref()
        .and_then(|a| a.get("timeFrame"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase()
}
