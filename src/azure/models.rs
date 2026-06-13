//! Structs serde para os payloads da API REST do Azure DevOps.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Operação de JSON Patch (`application/json-patch+json`) usada na criação e
/// atualização de work items.
#[derive(Debug, Clone, Serialize)]
pub struct JsonPatchOp {
    pub op: String,
    pub path: String,
    pub value: Value,
}

impl JsonPatchOp {
    /// Cria uma operação `add` para um campo (`/fields/{reference_name}`).
    pub fn add_field(reference_name: &str, value: impl Into<Value>) -> Self {
        JsonPatchOp {
            op: "add".to_string(),
            path: format!("/fields/{reference_name}"),
            value: value.into(),
        }
    }

    /// Cria uma operação `add` de relação (`/relations/-`), usada para vincular
    /// work items (pai/filho, related, predecessor/successor). `url` deve ser a
    /// URL do work item alvo; `comment` é anexado em `attributes` quando presente.
    pub fn add_relation(rel: &str, url: String, comment: Option<String>) -> Self {
        let mut value = json!({ "rel": rel, "url": url });
        if let Some(c) = comment {
            value["attributes"] = json!({ "comment": c });
        }
        JsonPatchOp {
            op: "add".to_string(),
            path: "/relations/-".to_string(),
            value,
        }
    }
}

/// Um work item retornado pela API (campos ficam em um mapa dinâmico).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkItem {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub rev: i64,
    #[serde(default)]
    pub fields: Value,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default, rename = "relations")]
    pub relations: Option<Value>,
}

/// Resposta de busca por múltiplos work items.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkItemList {
    #[serde(default)]
    pub value: Vec<WorkItem>,
}

/// Referência a um work item dentro do resultado de uma consulta WIQL.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkItemReference {
    pub id: i64,
    #[serde(default)]
    pub url: Option<String>,
}

/// Resultado de uma consulta WIQL.
#[derive(Debug, Clone, Deserialize)]
pub struct WiqlResult {
    #[serde(default, rename = "workItems")]
    pub work_items: Vec<WorkItemReference>,
}

/// Comentário de um work item.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Comment {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub text: String,
    #[serde(default, rename = "createdBy")]
    pub created_by: Option<Value>,
    #[serde(default, rename = "createdDate")]
    pub created_date: Option<String>,
}

/// Lista de comentários de um work item.
#[derive(Debug, Clone, Deserialize)]
pub struct CommentList {
    #[serde(default)]
    pub count: i64,
    #[serde(default)]
    pub comments: Vec<Comment>,
}

/// Uma iteração (sprint) configurada para o time.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Iteration {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub attributes: Option<Value>,
}

/// Lista de iterações do time.
#[derive(Debug, Clone, Deserialize)]
pub struct IterationList {
    #[serde(default)]
    pub value: Vec<Iteration>,
}

/// Relação entre uma iteração e um work item (usada ao listar itens da sprint).
#[derive(Debug, Clone, Deserialize)]
pub struct IterationWorkItemRelation {
    pub target: WorkItemReference,
}

/// Resposta de work items de uma iteração.
#[derive(Debug, Clone, Deserialize)]
pub struct IterationWorkItems {
    #[serde(default, rename = "workItemRelations")]
    pub work_item_relations: Vec<IterationWorkItemRelation>,
}

/// Estado possível de um tipo de work item (projeção enxuta).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkItemTypeState {
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

/// Projeção enxuta de um tipo de work item. A API retorna, para cada tipo, toda
/// a definição (campos, transições, ícones, helpText), o que infla a resposta em
/// centenas de KB. Aqui ficamos só com o essencial para o agente decidir.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkItemType {
    #[serde(default)]
    pub name: String,
    #[serde(rename = "referenceName", default)]
    pub reference_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(rename = "isDisabled", default)]
    pub is_disabled: bool,
    #[serde(default)]
    pub states: Vec<WorkItemTypeState>,
}

/// Envelope da resposta de `workitemtypes`.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkItemTypeList {
    #[serde(default)]
    pub value: Vec<WorkItemType>,
}

/// Referência de identidade aninhada no payload de membros do time.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct IdentityRef {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "displayName", default)]
    pub display_name: String,
    #[serde(rename = "uniqueName", default)]
    pub unique_name: Option<String>,
}

/// Item cru da resposta de membros do time (a identidade vem aninhada).
#[derive(Debug, Clone, Deserialize)]
pub struct TeamMemberRaw {
    #[serde(default)]
    pub identity: IdentityRef,
    #[serde(rename = "isTeamAdmin", default)]
    pub is_team_admin: bool,
}

/// Envelope da resposta de membros do time.
#[derive(Debug, Clone, Deserialize)]
pub struct TeamMemberList {
    #[serde(default)]
    pub value: Vec<TeamMemberRaw>,
}

/// Projeção enxuta de um membro do time (achata a identidade aninhada).
#[derive(Debug, Clone, Serialize)]
pub struct TeamMember {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "uniqueName", skip_serializing_if = "Option::is_none")]
    pub unique_name: Option<String>,
    #[serde(rename = "isAdmin")]
    pub is_admin: bool,
}

/// Projeção enxuta de um usuário retornado pela API de Identidades (people picker).
#[derive(Debug, Clone, Serialize)]
pub struct UserSummary {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "uniqueName", skip_serializing_if = "Option::is_none")]
    pub unique_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descriptor: Option<String>,
}

/// Mapeamento de uma coluna customizada do taskboard para um estado, por tipo.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskboardColumnMapping {
    #[serde(rename = "workItemType", default)]
    pub work_item_type: String,
    #[serde(default)]
    pub state: String,
}

/// Uma coluna customizada do taskboard de sprint (ex.: "Em Desenvolvimento").
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskboardColumn {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub order: i64,
    #[serde(default)]
    pub mappings: Vec<TaskboardColumnMapping>,
}

/// Configuração das colunas do taskboard ("Customize columns on taskboard").
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskboardColumns {
    #[serde(default)]
    pub columns: Vec<TaskboardColumn>,
    #[serde(rename = "isCustomized", default)]
    pub is_customized: bool,
    #[serde(rename = "isValid", default)]
    pub is_valid: bool,
}

/// Posição de um work item no taskboard: em qual coluna customizada ele está.
/// (A API também devolve `state`/`columnId`, ignorados — a coluna real vem em `column`.)
#[derive(Debug, Clone, Deserialize)]
pub struct TaskboardWorkItem {
    #[serde(rename = "workItemId", default)]
    pub work_item_id: i64,
    #[serde(default)]
    pub column: Option<String>,
}

/// Envelope da resposta de posições do taskboard.
#[derive(Debug, Clone, Deserialize)]
pub struct TaskboardWorkItemList {
    #[serde(default)]
    pub value: Vec<TaskboardWorkItem>,
}
