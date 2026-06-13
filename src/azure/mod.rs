//! Cliente HTTP para a API REST de Work Item Tracking do Azure DevOps.

pub mod models;

use anyhow::{bail, Context, Result};
use reqwest::{Client, Method, RequestBuilder};
use serde_json::{json, Value};

use crate::config::Config;
use models::{
    Comment, CommentList, Iteration, IterationList, IterationWorkItems, JsonPatchOp,
    TaskboardColumns, TaskboardWorkItem, TaskboardWorkItemList, TeamMember, TeamMemberList,
    UserSummary, WiqlResult, WorkItem, WorkItemList, WorkItemType, WorkItemTypeList,
};

/// Content-Type exigido pela API para operações de criação/atualização.
const JSON_PATCH: &str = "application/json-patch+json";

/// Conjunto enxuto de campos retornado quando o chamador não especifica `fields`.
/// Evita devolver TODOS os campos de cada work item (o payload cru é enorme).
const DEFAULT_FIELDS: &[&str] = &[
    "System.Id",
    "System.Title",
    "System.WorkItemType",
    "System.State",
    "System.AssignedTo",
    "System.Parent",
    "System.Tags",
    "System.IterationPath",
    "System.BoardColumn",
    "System.BoardColumnDone",
    "Microsoft.VSTS.Common.Priority",
    "Microsoft.VSTS.Scheduling.StoryPoints",
    "Microsoft.VSTS.Scheduling.RemainingWork",
];

/// Cliente que encapsula `reqwest::Client` + configuração.
#[derive(Clone)]
pub struct AzureClient {
    http: Client,
    config: Config,
}

impl AzureClient {
    /// Constrói o cliente HTTP. Falha se o cliente reqwest não puder ser criado.
    pub fn new(config: Config) -> Result<Self> {
        let http = Client::builder()
            .user_agent(concat!("ado-cli/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("falha ao construir o cliente HTTP")?;
        Ok(Self { http, config })
    }

    /// Prefixo das URLs de WIT: `{base}/{org}/{project}/_apis/wit`.
    fn wit_base(&self) -> String {
        format!(
            "{}/{}/{}/_apis/wit",
            self.config.base_url,
            urlencode(&self.config.organization),
            urlencode(&self.config.project),
        )
    }

    /// Monta um `RequestBuilder` já com autenticação Basic (PAT) aplicada.
    fn request(&self, method: Method, url: String) -> RequestBuilder {
        self.http
            .request(method, url)
            .basic_auth("", Some(&self.config.pat))
    }

    /// Envia a requisição e desserializa o JSON, propagando o corpo de erro da
    /// API em caso de status != 2xx.
    async fn send_json<T: serde::de::DeserializeOwned>(&self, req: RequestBuilder) -> Result<T> {
        let resp = req.send().await.context("falha na requisição HTTP")?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            bail!("Azure DevOps retornou {status}: {body}");
        }
        serde_json::from_str(&body)
            .with_context(|| format!("resposta inesperada da API (status {status}): {body}"))
    }

    /// Envia a requisição apenas validando o status (para endpoints que não
    /// retornam JSON útil, como o move de coluna do taskboard).
    async fn send_ok(&self, req: RequestBuilder) -> Result<()> {
        let resp = req.send().await.context("falha na requisição HTTP")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            bail!("Azure DevOps retornou {status}: {body}");
        }
        Ok(())
    }

    // ---- Work items -------------------------------------------------------

    /// Executa uma consulta WIQL e retorna os work items referenciados. Se `wiql`
    /// for `None`, usa uma consulta padrão com os itens mais recentes do projeto.
    /// `fields` (opcional) limita os campos retornados, economizando contexto.
    pub async fn query_work_items(
        &self,
        wiql: Option<&str>,
        fields: Option<&[String]>,
    ) -> Result<Vec<WorkItem>> {
        let query = wiql.map(str::to_string).unwrap_or_else(default_wiql);
        let url = format!(
            "{}/wiql?api-version={}",
            self.wit_base(),
            self.config.api_version
        );
        let result: WiqlResult = self
            .send_json(self.request(Method::POST, url).json(&json!({ "query": query })))
            .await?;

        let ids: Vec<i64> = result.work_items.iter().map(|w| w.id).take(200).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        self.get_work_items(&ids, fields).await
    }

    /// Busca vários work items por id. `fields` (opcional) restringe os campos
    /// retornados (reference names); quando `None`, retorna os campos padrão.
    pub async fn get_work_items(
        &self,
        ids: &[i64],
        fields: Option<&[String]>,
    ) -> Result<Vec<WorkItem>> {
        let ids_csv = ids
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let mut url = format!(
            "{}/workitems?ids={}&api-version={}",
            self.wit_base(),
            ids_csv,
            self.config.api_version
        );
        // Restringe os campos: usa os solicitados ou, na ausência deles, um
        // conjunto padrão enxuto — devolver todos os campos infla demais a resposta.
        url.push_str("&fields=");
        match fields.filter(|f| !f.is_empty()) {
            Some(f) => url.push_str(&f.join(",")),
            None => url.push_str(&DEFAULT_FIELDS.join(",")),
        }
        let list: WorkItemList = self.send_json(self.request(Method::GET, url)).await?;
        let mut items = list.value;
        for item in &mut items {
            compact_identity_fields(&mut item.fields);
        }
        Ok(items)
    }

    /// Busca vários work items COM suas relações (`$expand=relations`), em lote.
    /// Usado para montar árvores de hierarquia/dependência. Identidades compactadas.
    pub async fn get_work_items_with_relations(&self, ids: &[i64]) -> Result<Vec<WorkItem>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids_csv = ids
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let url = format!(
            "{}/workitems?ids={}&$expand=relations&api-version={}",
            self.wit_base(),
            ids_csv,
            self.config.api_version
        );
        let list: WorkItemList = self.send_json(self.request(Method::GET, url)).await?;
        let mut items = list.value;
        for item in &mut items {
            compact_identity_fields(&mut item.fields);
        }
        Ok(items)
    }

    /// Detalhes de um work item incluindo as relações (`$expand=relations`).
    /// Evita `$expand=all`, que ainda agrega `_links` e metadados volumosos.
    pub async fn get_work_item(&self, id: i64) -> Result<WorkItem> {
        let url = format!(
            "{}/workitems/{}?$expand=relations&api-version={}",
            self.wit_base(),
            id,
            self.config.api_version
        );
        let mut item: WorkItem = self.send_json(self.request(Method::GET, url)).await?;
        compact_identity_fields(&mut item.fields);
        Ok(item)
    }

    /// Cria um work item do tipo informado a partir de operações de patch.
    pub async fn create_work_item(
        &self,
        work_item_type: &str,
        ops: &[JsonPatchOp],
    ) -> Result<WorkItem> {
        let url = format!(
            "{}/workitems/${}?api-version={}",
            self.wit_base(),
            urlencode(work_item_type),
            self.config.api_version
        );
        let req = self
            .request(Method::POST, url)
            .header(reqwest::header::CONTENT_TYPE, JSON_PATCH)
            .json(ops);
        self.send_json(req).await
    }

    /// Atualiza um work item existente com operações de patch.
    pub async fn update_work_item(&self, id: i64, ops: &[JsonPatchOp]) -> Result<WorkItem> {
        if ops.is_empty() {
            bail!("nenhum campo informado para atualização");
        }
        let url = format!(
            "{}/workitems/{}?api-version={}",
            self.wit_base(),
            id,
            self.config.api_version
        );
        let req = self
            .request(Method::PATCH, url)
            .header(reqwest::header::CONTENT_TYPE, JSON_PATCH)
            .json(ops);
        self.send_json(req).await
    }

    /// URL de API (org-level) de um work item, usada como alvo de relações.
    pub fn work_item_api_url(&self, id: i64) -> String {
        format!(
            "{}/{}/_apis/wit/workItems/{}",
            self.config.base_url,
            urlencode(&self.config.organization),
            id
        )
    }

    /// Adiciona uma relação (link) de `id` para `target_id`. `rel` é o reference
    /// name do tipo de link (ex.: "System.LinkTypes.Hierarchy-Reverse" para pai).
    pub async fn add_link(
        &self,
        id: i64,
        target_id: i64,
        rel: &str,
        comment: Option<String>,
    ) -> Result<WorkItem> {
        let op = JsonPatchOp::add_relation(rel, self.work_item_api_url(target_id), comment);
        self.update_work_item(id, &[op]).await
    }

    /// Substitui o conjunto de tags (`System.Tags`) de um work item.
    pub async fn set_tags(&self, id: i64, tags: &[String]) -> Result<WorkItem> {
        let value = tags.join("; ");
        let ops = vec![JsonPatchOp::add_field("System.Tags", value)];
        self.update_work_item(id, &ops).await
    }

    /// Tags atuais de um work item (lê `System.Tags`, separadas por `;`).
    pub async fn get_tags(&self, id: i64) -> Result<Vec<String>> {
        let item = self.get_work_item(id).await?;
        Ok(parse_tags(
            item.fields.get("System.Tags").and_then(|v| v.as_str()),
        ))
    }

    /// Lista os tipos de work item válidos para o projeto (projeção enxuta:
    /// nome, reference name, descrição, cor e estados — sem campos/transições).
    pub async fn list_work_item_types(&self) -> Result<Vec<WorkItemType>> {
        let url = format!(
            "{}/workitemtypes?api-version={}",
            self.wit_base(),
            self.config.api_version
        );
        let list: WorkItemTypeList = self.send_json(self.request(Method::GET, url)).await?;
        Ok(list.value)
    }

    // ---- Pessoas / usuários ----------------------------------------------

    /// Host das APIs de identidade. Na nuvem fica em `vssps.dev.azure.com`; em
    /// on-prem, as APIs de identidade ficam no próprio host da coleção.
    fn identities_host(&self) -> String {
        if self.config.base_url.contains("://dev.azure.com") {
            self.config
                .base_url
                .replace("://dev.azure.com", "://vssps.dev.azure.com")
        } else {
            self.config.base_url.clone()
        }
    }

    /// Lista os membros do time configurado (projeção: id, displayName,
    /// uniqueName, isAdmin). Use o `uniqueName` em filtros de System.AssignedTo.
    pub async fn list_team_members(&self) -> Result<Vec<TeamMember>> {
        let url = format!(
            "{}/{}/_apis/projects/{}/teams/{}/members?api-version={}",
            self.config.base_url,
            urlencode(&self.config.organization),
            urlencode(&self.config.project),
            urlencode(&self.config.team),
            self.config.api_version
        );
        let list: TeamMemberList = self.send_json(self.request(Method::GET, url)).await?;
        Ok(list
            .value
            .into_iter()
            .map(|m| TeamMember {
                id: m.identity.id,
                display_name: m.identity.display_name,
                unique_name: m.identity.unique_name,
                is_admin: m.is_team_admin,
            })
            .collect())
    }

    /// Pesquisa usuários da organização por nome ou e-mail (people picker),
    /// via API de Identidades. Projeção: id, displayName, uniqueName, descriptor.
    pub async fn search_users(&self, query: &str) -> Result<Vec<UserSummary>> {
        let url = format!(
            "{}/{}/_apis/identities?searchFilter=General&filterValue={}&queryMembership=None&api-version={}",
            self.identities_host(),
            urlencode(&self.config.organization),
            urlencode(query),
            self.config.api_version
        );
        let v: Value = self.send_json(self.request(Method::GET, url)).await?;
        let users = v
            .get("value")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().map(project_identity).collect())
            .unwrap_or_default();
        Ok(users)
    }

    // ---- Iterações / sprint ----------------------------------------------

    /// Prefixo das URLs de Work (team-scoped): `{base}/{org}/{project}/{team}/_apis/work`.
    fn work_base(&self) -> String {
        format!(
            "{}/{}/{}/{}/_apis/work",
            self.config.base_url,
            urlencode(&self.config.organization),
            urlencode(&self.config.project),
            urlencode(&self.config.team),
        )
    }

    /// Lista as iterações do time. `timeframe` opcional ("current") filtra a
    /// sprint corrente.
    pub async fn list_iterations(&self, timeframe: Option<&str>) -> Result<Vec<Iteration>> {
        let mut url = format!(
            "{}/teamsettings/iterations?api-version={}",
            self.work_base(),
            self.config.api_version
        );
        if let Some(tf) = timeframe {
            url.push_str("&$timeframe=");
            url.push_str(tf);
        }
        let list: IterationList = self.send_json(self.request(Method::GET, url)).await?;
        Ok(list.value)
    }

    /// Iteração atual (sprint corrente) do time, se houver.
    pub async fn current_iteration(&self) -> Result<Option<Iteration>> {
        Ok(self.list_iterations(Some("current")).await?.into_iter().next())
    }

    /// Ids dos work items associados a uma iteração.
    pub async fn iteration_work_item_ids(&self, iteration_id: &str) -> Result<Vec<i64>> {
        let url = format!(
            "{}/teamsettings/iterations/{}/workitems?api-version={}",
            self.work_base(),
            urlencode(iteration_id),
            self.config.api_version
        );
        let res: IterationWorkItems = self.send_json(self.request(Method::GET, url)).await?;
        Ok(res.work_item_relations.iter().map(|r| r.target.id).collect())
    }

    /// Define a iteração (sprint) de um work item via `System.IterationPath`.
    pub async fn set_iteration_path(&self, id: i64, path: &str) -> Result<WorkItem> {
        let ops = vec![JsonPatchOp::add_field("System.IterationPath", path)];
        self.update_work_item(id, &ops).await
    }

    /// Resolve o `IterationPath` do backlog do time (a raiz), lendo o
    /// `backlogIteration` das configurações do time. É para onde itens sem sprint
    /// pertencem; usado por `move_to_backlog`.
    pub async fn backlog_iteration_path(&self) -> Result<String> {
        let url = format!(
            "{}/teamsettings?api-version={}",
            self.work_base(),
            self.config.api_version
        );
        let v: Value = self.send_json(self.request(Method::GET, url)).await?;
        let backlog = v.get("backlogIteration");
        let name = backlog
            .and_then(|b| b.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let path = backlog
            .and_then(|b| b.get("path"))
            .and_then(Value::as_str)
            .unwrap_or("");
        // A raiz do backlog vem com path vazio e name = nome do projeto; o
        // IterationPath usado nos itens para a raiz é justamente esse name.
        let iteration_path = if path.trim().is_empty() {
            name.to_string()
        } else {
            path.to_string()
        };
        if iteration_path.is_empty() {
            bail!("não foi possível resolver o backlog do time (backlogIteration vazio)");
        }
        Ok(iteration_path)
    }

    // ---- Taskboard (sprint) ----------------------------------------------

    /// As APIs de taskboard usam uma versão preview baseada na versão configurada.
    fn taskboard_api_version(&self) -> String {
        format!("{}-preview.1", self.config.api_version)
    }

    /// Colunas customizadas do taskboard de sprint (config + mapeamentos
    /// coluna→estado por tipo). É o "Customize columns on taskboard".
    pub async fn get_taskboard_columns(&self) -> Result<TaskboardColumns> {
        let url = format!(
            "{}/taskboardcolumns?api-version={}",
            self.work_base(),
            self.taskboard_api_version()
        );
        self.send_json(self.request(Method::GET, url)).await
    }

    /// Posição de cada work item no taskboard da iteração (em qual coluna
    /// customizada ele está). Necessário porque várias colunas podem mapear o
    /// mesmo estado — só esta API distingue a coluna real de cada item.
    pub async fn taskboard_work_items(&self, iteration_id: &str) -> Result<Vec<TaskboardWorkItem>> {
        let url = format!(
            "{}/taskboardworkitems/{}?api-version={}",
            self.work_base(),
            urlencode(iteration_id),
            self.taskboard_api_version()
        );
        let list: TaskboardWorkItemList = self.send_json(self.request(Method::GET, url)).await?;
        Ok(list.value)
    }

    /// Move um work item para uma coluna específica do taskboard de uma iteração.
    /// Funciona inclusive entre colunas que compartilham o mesmo estado (a coluna
    /// não é um campo do work item — é persistida pelo serviço de taskboard).
    /// `new_column` é o NOME exato da coluna (ver `get_taskboard_columns`).
    pub async fn set_taskboard_column(
        &self,
        iteration_id: &str,
        work_item_id: i64,
        new_column: &str,
    ) -> Result<()> {
        let url = format!(
            "{}/taskboardworkitems/{}/{}?api-version={}",
            self.work_base(),
            urlencode(iteration_id),
            work_item_id,
            self.taskboard_api_version()
        );
        let req = self
            .request(Method::PATCH, url)
            .json(&json!({ "newColumn": new_column }));
        self.send_ok(req).await
    }

    // ---- Comentários ------------------------------------------------------

    /// A API de comentários usa uma versão preview baseada na versão configurada.
    fn comments_api_version(&self) -> String {
        format!("{}-preview.4", self.config.api_version)
    }

    /// Lista os comentários de um work item.
    pub async fn list_comments(&self, id: i64) -> Result<CommentList> {
        let url = format!(
            "{}/workItems/{}/comments?api-version={}",
            self.wit_base(),
            id,
            self.comments_api_version()
        );
        self.send_json(self.request(Method::GET, url)).await
    }

    /// Adiciona um comentário a um work item.
    pub async fn add_comment(&self, id: i64, text: &str) -> Result<Comment> {
        let url = format!(
            "{}/workItems/{}/comments?api-version={}",
            self.wit_base(),
            id,
            self.comments_api_version()
        );
        let req = self
            .request(Method::POST, url)
            .json(&json!({ "text": text }));
        self.send_json(req).await
    }
}

/// Consulta WIQL padrão: itens do projeto mais recentemente alterados.
fn default_wiql() -> String {
    "SELECT [System.Id] FROM WorkItems \
     WHERE [System.TeamProject] = @project \
     ORDER BY [System.ChangedDate] DESC"
        .to_string()
}

/// Compacta, in-place, os campos de identidade de um work item. A API expande
/// campos como `System.AssignedTo`/`System.CreatedBy` num objeto enorme (com
/// `_links`, `avatar`, `imageUrl`, `descriptor`, `url`...). Aqui reduzimos cada
/// um para uma string curta "Nome <email>", preservando o que importa.
fn compact_identity_fields(fields: &mut Value) {
    let Some(map) = fields.as_object_mut() else {
        return;
    };
    for value in map.values_mut() {
        if let Some(compact) = identity_to_string(value) {
            *value = Value::String(compact);
        }
    }
}

/// Se `value` for um objeto de identidade (tem `displayName`), retorna a forma
/// compacta "Nome <uniqueName>"; caso contrário, `None` (valor mantido como está).
fn identity_to_string(value: &Value) -> Option<String> {
    let obj = value.as_object()?;
    let display = obj.get("displayName")?.as_str()?;
    match obj.get("uniqueName").and_then(Value::as_str) {
        Some(unique) if !unique.is_empty() => Some(format!("{display} <{unique}>")),
        _ => Some(display.to_string()),
    }
}

/// Projeta uma identidade da API de Identidades em um `UserSummary` enxuto.
/// O nome vem de `providerDisplayName` (ou `properties.DisplayName`) e o
/// `uniqueName` de `properties.Mail` (ou `properties.Account`).
fn project_identity(v: &Value) -> UserSummary {
    let id = v
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let display_name = v
        .get("providerDisplayName")
        .and_then(Value::as_str)
        .or_else(|| v.pointer("/properties/DisplayName/$value").and_then(Value::as_str))
        .unwrap_or_default()
        .to_string();
    let unique_name = v
        .pointer("/properties/Mail/$value")
        .and_then(Value::as_str)
        .or_else(|| v.pointer("/properties/Account/$value").and_then(Value::as_str))
        .map(str::to_string)
        .filter(|s| !s.is_empty());
    let descriptor = v.get("descriptor").and_then(Value::as_str).map(str::to_string);
    UserSummary {
        id,
        display_name,
        unique_name,
        descriptor,
    }
}

/// Separa a string de tags do Azure (`System.Tags`, ex.: "a; b; c") em itens
/// limpos, ignorando vazios.
fn parse_tags(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or_default()
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Percent-encoding mínimo para segmentos de path (org/projeto/tipo podem ter
/// espaços e outros caracteres). Mantém caracteres não reservados.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> AzureClient {
        AzureClient::new(Config {
            pat: "x".into(),
            organization: "contoso".into(),
            project: "My Project".into(),
            team: "My Project Team".into(),
            base_url: "https://dev.azure.com".into(),
            api_version: "7.1".into(),
        })
        .unwrap()
    }

    #[test]
    fn wit_base_url_com_encoding() {
        let c = client();
        assert_eq!(
            c.wit_base(),
            "https://dev.azure.com/contoso/My%20Project/_apis/wit"
        );
    }

    #[test]
    fn work_base_url_com_team() {
        assert_eq!(
            client().work_base(),
            "https://dev.azure.com/contoso/My%20Project/My%20Project%20Team/_apis/work"
        );
    }

    #[test]
    fn work_item_api_url_org_level() {
        assert_eq!(
            client().work_item_api_url(42),
            "https://dev.azure.com/contoso/_apis/wit/workItems/42"
        );
    }

    #[test]
    fn parse_tags_separa_e_limpa() {
        assert_eq!(
            parse_tags(Some("alpha; beta ;; gamma")),
            vec!["alpha", "beta", "gamma"]
        );
        assert!(parse_tags(None).is_empty());
    }

    #[test]
    fn comments_api_version_preview() {
        assert_eq!(client().comments_api_version(), "7.1-preview.4");
    }

    #[test]
    fn patch_op_add_field() {
        let op = JsonPatchOp::add_field("System.Title", "Olá");
        assert_eq!(op.op, "add");
        assert_eq!(op.path, "/fields/System.Title");
        assert_eq!(op.value, serde_json::json!("Olá"));
    }

    #[test]
    fn urlencode_espacos() {
        assert_eq!(urlencode("My Project"), "My%20Project");
        assert_eq!(urlencode("Task"), "Task");
    }

    #[test]
    fn compacta_campo_identidade() {
        let mut fields = json!({
            "System.Title": "Tarefa",
            "System.AssignedTo": {
                "displayName": "Fulano de Tal",
                "uniqueName": "fulano@empresa.com",
                "_links": { "avatar": { "href": "https://x/avatar" } },
                "imageUrl": "https://x/image"
            }
        });
        compact_identity_fields(&mut fields);
        assert_eq!(fields["System.AssignedTo"], json!("Fulano de Tal <fulano@empresa.com>"));
        // Campos não-identidade permanecem intactos.
        assert_eq!(fields["System.Title"], json!("Tarefa"));
    }

    #[test]
    fn identidade_sem_unique_name_e_nao_identidade() {
        assert_eq!(
            identity_to_string(&json!({ "displayName": "Beltrano" })),
            Some("Beltrano".to_string())
        );
        assert_eq!(identity_to_string(&json!("apenas texto")), None);
        assert_eq!(identity_to_string(&json!({ "outro": "campo" })), None);
    }

    #[test]
    fn patch_op_add_relation() {
        let op = JsonPatchOp::add_relation(
            "System.LinkTypes.Hierarchy-Reverse",
            "https://dev.azure.com/contoso/_apis/wit/workItems/42".to_string(),
            Some("pai".to_string()),
        );
        assert_eq!(op.op, "add");
        assert_eq!(op.path, "/relations/-");
        assert_eq!(op.value["rel"], "System.LinkTypes.Hierarchy-Reverse");
        assert_eq!(op.value["attributes"]["comment"], "pai");
    }
}
