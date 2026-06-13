//! Definição da interface de linha de comando (clap derive).
//!
//! Importante: NÃO há flags de configuração (PAT, projeto, etc.) — toda a
//! configuração vem do arquivo `.env` (ver `config.rs`). Os argumentos aqui são
//! apenas os parâmetros das operações.

use clap::{Parser, Subcommand};

use crate::ops::LinkType;

#[derive(Parser, Debug)]
#[command(
    name = "ado-cli",
    version,
    about = "CLI para work items de um board do Azure DevOps (saída JSON)",
    long_about = "Manipula work items de um board do Azure DevOps. Toda a configuração \
(AZDO_PAT, AZDO_PROJECT, AZDO_TEAM, AZDO_BASE_URL, AZDO_API_VERSION) vem do arquivo \
.env no diretório atual, com fallback para variáveis de ambiente do SO. A saída é \
sempre JSON no stdout."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
// A variante `Create` tem muitos campos opcionais; boxear quebraria o derive do
// clap sem ganho real (a enum é construída uma vez, no parse).
#[allow(clippy::large_enum_variant)]
pub enum Command {
    /// Consulta work items via WIQL (sem --wiql, lista os mais recentes ABERTOS).
    Query {
        /// Texto WIQL completo. Quando informado, é respeitado como está.
        #[arg(long)]
        wiql: Option<String>,
        /// Sem --wiql, inclui também os fechados. Por padrão, só abertos.
        /// (Ignorado quando --wiql é informado.)
        #[arg(long)]
        include_closed: bool,
        /// Campos a retornar (reference names), separados por vírgula.
        #[arg(long, value_delimiter = ',')]
        fields: Vec<String>,
    },
    /// Detalhes completos de um work item (inclui relations).
    Get {
        /// Id do work item.
        id: i64,
    },
    /// Relações de um work item: árvore de filhos, pais e dependências.
    Links {
        /// Id do work item.
        id: i64,
    },
    /// Cria um novo work item (Task, Bug, User Story, ...).
    Create {
        /// Tipo do work item (ex.: Task, Bug, "User Story").
        #[arg(long = "type")]
        work_item_type: String,
        /// Título (System.Title).
        #[arg(long)]
        title: String,
        /// Descrição (System.Description, aceita HTML).
        #[arg(long)]
        description: Option<String>,
        /// Responsável (System.AssignedTo): email ou display name.
        #[arg(long)]
        assigned_to: Option<String>,
        /// Estado inicial (System.State).
        #[arg(long)]
        state: Option<String>,
        /// Area path (System.AreaPath).
        #[arg(long)]
        area_path: Option<String>,
        /// Iteration path (System.IterationPath).
        #[arg(long)]
        iteration_path: Option<String>,
        /// Tags (System.Tags), separadas por vírgula.
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Prioridade (Microsoft.VSTS.Common.Priority).
        #[arg(long)]
        priority: Option<i64>,
        /// Story points (Microsoft.VSTS.Scheduling.StoryPoints).
        #[arg(long)]
        story_points: Option<f64>,
        /// Critérios de aceite (Microsoft.VSTS.Common.AcceptanceCriteria).
        #[arg(long)]
        acceptance_criteria: Option<String>,
        /// Passos de reprodução, p/ bugs (Microsoft.VSTS.TCM.ReproSteps).
        #[arg(long)]
        repro_steps: Option<String>,
        /// Estimativa original (Microsoft.VSTS.Scheduling.OriginalEstimate).
        #[arg(long)]
        original_estimate: Option<f64>,
        /// Trabalho restante (Microsoft.VSTS.Scheduling.RemainingWork).
        #[arg(long)]
        remaining_work: Option<f64>,
        /// Id do pai: linka o novo item como filho ao criar.
        #[arg(long)]
        parent_id: Option<i64>,
    },
    /// Atualiza campos arbitrários (reference names).
    Update {
        /// Id do work item.
        id: i64,
        /// Campo a atualizar no formato reference_name=valor (repetível).
        #[arg(long = "set", value_name = "REF=VALOR")]
        set: Vec<String>,
        /// Objeto JSON com os campos a atualizar (ex.: '{"System.Title":"x"}').
        /// Mesclado com os --set; sem --set, lido do stdin se omitido.
        #[arg(long)]
        json: Option<String>,
    },
    /// Altera o estado (System.State) de um work item.
    SetState {
        /// Id do work item.
        id: i64,
        /// Novo estado (ex.: Active, Resolved, Closed).
        state: String,
    },
    /// Atribui um work item a uma pessoa (System.AssignedTo); vazio remove.
    Assign {
        /// Id do work item.
        id: i64,
        /// Responsável: email ou display name. String vazia remove.
        assigned_to: String,
    },
    /// Move uma task para uma coluna do TASKBOARD da sprint.
    SetTaskboardColumn {
        /// Id do work item (normalmente uma Task).
        id: i64,
        /// Nome EXATO da coluna destino (ver taskboard-columns).
        #[arg(long)]
        column: String,
        /// Id (uuid) da iteração; sem isso, usa a sprint atual.
        #[arg(long)]
        iteration_id: Option<String>,
    },
    /// Vincula dois work items (pai/filho, related, predecessor/successor).
    AddLink {
        /// Work item de origem.
        id: i64,
        /// Work item alvo.
        target_id: i64,
        /// Tipo de vínculo.
        #[arg(long, value_enum)]
        link_type: LinkType,
        /// Comentário opcional anexado ao link.
        #[arg(long)]
        comment: Option<String>,
    },
    /// Reordena um item no backlog (StackRank por padrão; menor = mais acima).
    SetBacklogPriority {
        /// Id do work item.
        id: i64,
        /// Valor de ordenação no backlog.
        priority: f64,
        /// Campo de ordenação (default: Microsoft.VSTS.Common.StackRank).
        #[arg(long)]
        field: Option<String>,
    },
    /// Lista os comentários de um work item.
    ListComments {
        /// Id do work item.
        id: i64,
    },
    /// Adiciona um comentário a um work item.
    AddComment {
        /// Id do work item.
        id: i64,
        /// Texto do comentário.
        text: String,
    },
    /// Lista as iterações (sprints) do time.
    ListIterations {
        /// Filtro temporal: current, past ou future.
        #[arg(long)]
        timeframe: Option<String>,
        /// Inclui sprints já encerradas.
        #[arg(long)]
        include_closed: bool,
    },
    /// Sprint atual do time (por padrão só os IDs; --fields hidrata).
    CurrentSprint {
        /// Campos a retornar (reference names), separados por vírgula.
        #[arg(long, value_delimiter = ',')]
        fields: Vec<String>,
    },
    /// Work items atribuídos ao dono do PAT (@Me). Por padrão, só os ABERTOS.
    MyWorkItems {
        /// Limita à sprint atual do time.
        #[arg(long)]
        only_current_sprint: bool,
        /// Inclui também os itens fechados (estados terminais). Por padrão, só abertos.
        #[arg(long)]
        include_closed: bool,
        /// Campos a retornar (reference names), separados por vírgula.
        #[arg(long, value_delimiter = ',')]
        fields: Vec<String>,
    },
    /// Colunas customizadas do taskboard de sprint (config + mapeamentos).
    TaskboardColumns,
    /// Visão do taskboard da sprint atual (itens agrupados por coluna).
    Taskboard {
        /// Campos a retornar (reference names), separados por vírgula.
        #[arg(long, value_delimiter = ',')]
        fields: Vec<String>,
    },
    /// Move um work item para uma iteração/sprint (define System.IterationPath).
    MoveToIteration {
        /// Id do work item.
        id: i64,
        /// Iteration path destino (ex.: "Loja\\Sprint 5").
        #[arg(long)]
        iteration_path: String,
    },
    /// Devolve um work item ao BACKLOG do time.
    MoveToBacklog {
        /// Id do work item.
        id: i64,
    },
    /// Move um work item para a sprint atual do time.
    MoveToCurrentSprint {
        /// Id do work item.
        id: i64,
    },
    /// Decompõe um pai criando várias sub-tasks já vinculadas a ele.
    CreateChildTasks {
        /// Work item pai a decompor.
        #[arg(long)]
        parent_id: i64,
        /// Array JSON de sub-tasks (cada uma com title e campos opcionais).
        /// Se omitido, lido do stdin.
        #[arg(long)]
        json: Option<String>,
    },
    /// Adiciona tags a um work item (preserva as existentes).
    AddTags {
        /// Id do work item.
        id: i64,
        /// Tags a adicionar, separadas por vírgula.
        #[arg(long, value_delimiter = ',', required = true)]
        tags: Vec<String>,
    },
    /// Remove tags de um work item (mantém as demais).
    RemoveTags {
        /// Id do work item.
        id: i64,
        /// Tags a remover, separadas por vírgula.
        #[arg(long, value_delimiter = ',', required = true)]
        tags: Vec<String>,
    },
    /// Lista os tipos de work item válidos do projeto (e seus estados).
    ListWorkItemTypes,
    /// Lista os membros do time configurado.
    ListTeamMembers,
    /// Pesquisa usuários da organização por nome ou e-mail (people picker).
    SearchUsers {
        /// Texto para buscar: nome de exibição ou e-mail (ou parte deles).
        query: String,
    },
    /// Instala a skill do Claude Code em ./.claude/skills/ (sobrescreve).
    Skill,
}
