//! Command-line interface definition (clap derive).
//!
//! Important: there are NO configuration flags (PAT, project, etc.) — all
//! configuration comes from the `.env` file (see `config.rs`). The arguments here
//! are only the operation parameters.

use clap::{Parser, Subcommand};

use crate::ops::LinkType;

#[derive(Parser, Debug)]
#[command(
    name = "ado-cli",
    version,
    about = "CLI for work items on an Azure DevOps board (JSON output)",
    long_about = "Manages work items on an Azure DevOps board. All configuration \
(AZDO_PAT, AZDO_PROJECT, AZDO_TEAM, AZDO_BASE_URL, AZDO_API_VERSION) comes from the \
.env file in the current directory, with a fallback to OS environment variables. \
Output is always JSON on stdout."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
// The `Create` variant has many optional fields; boxing would break clap's derive
// with no real gain (the enum is built once, at parse time).
#[allow(clippy::large_enum_variant)]
pub enum Command {
    /// Query work items via WIQL (without --wiql, lists the most recent OPEN ones).
    Query {
        /// Full WIQL text. When provided, it is respected as-is.
        #[arg(long)]
        wiql: Option<String>,
        /// Without --wiql, also include closed ones. By default, only open.
        /// (Ignored when --wiql is provided.)
        #[arg(long)]
        include_closed: bool,
        /// Fields to return (reference names), comma-separated.
        #[arg(long, value_delimiter = ',')]
        fields: Vec<String>,
    },
    /// Full details of a work item (includes relations).
    Get {
        /// Work item id.
        id: i64,
    },
    /// Relations of a work item: child tree, parents, and dependencies.
    Links {
        /// Work item id.
        id: i64,
    },
    /// Create a new work item (Task, Bug, User Story, ...).
    Create {
        /// Work item type (e.g. Task, Bug, "User Story").
        #[arg(long = "type")]
        work_item_type: String,
        /// Title (System.Title).
        #[arg(long)]
        title: String,
        /// Description (System.Description, accepts HTML).
        #[arg(long)]
        description: Option<String>,
        /// Assignee (System.AssignedTo): email or display name.
        #[arg(long)]
        assigned_to: Option<String>,
        /// Initial state (System.State).
        #[arg(long)]
        state: Option<String>,
        /// Area path (System.AreaPath).
        #[arg(long)]
        area_path: Option<String>,
        /// Iteration path (System.IterationPath).
        #[arg(long)]
        iteration_path: Option<String>,
        /// Tags (System.Tags), comma-separated.
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Priority (Microsoft.VSTS.Common.Priority).
        #[arg(long)]
        priority: Option<i64>,
        /// Story points (Microsoft.VSTS.Scheduling.StoryPoints).
        #[arg(long)]
        story_points: Option<f64>,
        /// Acceptance criteria (Microsoft.VSTS.Common.AcceptanceCriteria).
        #[arg(long)]
        acceptance_criteria: Option<String>,
        /// Repro steps, for bugs (Microsoft.VSTS.TCM.ReproSteps).
        #[arg(long)]
        repro_steps: Option<String>,
        /// Original estimate (Microsoft.VSTS.Scheduling.OriginalEstimate).
        #[arg(long)]
        original_estimate: Option<f64>,
        /// Remaining work (Microsoft.VSTS.Scheduling.RemainingWork).
        #[arg(long)]
        remaining_work: Option<f64>,
        /// Parent id: links the new item as a child on creation.
        #[arg(long)]
        parent_id: Option<i64>,
    },
    /// Update arbitrary fields (reference names).
    Update {
        /// Work item id.
        id: i64,
        /// Field to update in reference_name=value format (repeatable).
        #[arg(long = "set", value_name = "REF=VALUE")]
        set: Vec<String>,
        /// JSON object with the fields to update (e.g. '{"System.Title":"x"}').
        /// Merged with --set; without --set, read from stdin if omitted.
        #[arg(long)]
        json: Option<String>,
    },
    /// Change the state (System.State) of a work item.
    SetState {
        /// Work item id.
        id: i64,
        /// New state (e.g. Active, Resolved, Closed).
        state: String,
    },
    /// Assign a work item to a person (System.AssignedTo); empty unassigns.
    Assign {
        /// Work item id.
        id: i64,
        /// Assignee: email or display name. Empty string unassigns.
        assigned_to: String,
    },
    /// Move a task to a sprint TASKBOARD column.
    SetTaskboardColumn {
        /// Work item id (usually a Task).
        id: i64,
        /// EXACT name of the destination column (see taskboard-columns).
        #[arg(long)]
        column: String,
        /// Iteration id (uuid); without it, uses the current sprint.
        #[arg(long)]
        iteration_id: Option<String>,
    },
    /// Link two work items (parent/child, related, predecessor/successor).
    AddLink {
        /// Source work item.
        id: i64,
        /// Target work item.
        target_id: i64,
        /// Link type.
        #[arg(long, value_enum)]
        link_type: LinkType,
        /// Optional comment attached to the link.
        #[arg(long)]
        comment: Option<String>,
    },
    /// Reorder an item in the backlog (StackRank by default; lower = higher up).
    SetBacklogPriority {
        /// Work item id.
        id: i64,
        /// Ordering value in the backlog.
        priority: f64,
        /// Ordering field (default: Microsoft.VSTS.Common.StackRank).
        #[arg(long)]
        field: Option<String>,
    },
    /// List the comments of a work item.
    ListComments {
        /// Work item id.
        id: i64,
    },
    /// Add a comment to a work item.
    AddComment {
        /// Work item id.
        id: i64,
        /// Comment text.
        text: String,
    },
    /// List the team's iterations (sprints).
    ListIterations {
        /// Time filter: current, past, or future.
        #[arg(long)]
        timeframe: Option<String>,
        /// Include already-closed sprints.
        #[arg(long)]
        include_closed: bool,
    },
    /// The team's current sprint (only IDs by default; --fields hydrates).
    CurrentSprint {
        /// Fields to return (reference names), comma-separated.
        #[arg(long, value_delimiter = ',')]
        fields: Vec<String>,
    },
    /// Work items assigned to the PAT owner (@Me). By default, only OPEN ones.
    MyWorkItems {
        /// Limit to the team's current sprint.
        #[arg(long)]
        only_current_sprint: bool,
        /// Also include closed items (terminal states). By default, only open.
        #[arg(long)]
        include_closed: bool,
        /// Fields to return (reference names), comma-separated.
        #[arg(long, value_delimiter = ',')]
        fields: Vec<String>,
    },
    /// Sprint taskboard custom columns (config + mappings).
    TaskboardColumns,
    /// Current sprint taskboard view (items grouped by column).
    Taskboard {
        /// Fields to return (reference names), comma-separated.
        #[arg(long, value_delimiter = ',')]
        fields: Vec<String>,
    },
    /// Move a work item to an iteration/sprint (sets System.IterationPath).
    MoveToIteration {
        /// Work item id.
        id: i64,
        /// Destination iteration path (e.g. "Store\\Sprint 5").
        #[arg(long)]
        iteration_path: String,
    },
    /// Return a work item to the team's BACKLOG.
    MoveToBacklog {
        /// Work item id.
        id: i64,
    },
    /// Move a work item to the team's current sprint.
    MoveToCurrentSprint {
        /// Work item id.
        id: i64,
    },
    /// Decompose a parent by creating several sub-tasks already linked to it.
    CreateChildTasks {
        /// Parent work item to decompose.
        #[arg(long)]
        parent_id: i64,
        /// JSON array of sub-tasks (each with title and optional fields).
        /// If omitted, read from stdin.
        #[arg(long)]
        json: Option<String>,
    },
    /// Add tags to a work item (preserves existing ones).
    AddTags {
        /// Work item id.
        id: i64,
        /// Tags to add, comma-separated.
        #[arg(long, value_delimiter = ',', required = true)]
        tags: Vec<String>,
    },
    /// Remove tags from a work item (keeps the others).
    RemoveTags {
        /// Work item id.
        id: i64,
        /// Tags to remove, comma-separated.
        #[arg(long, value_delimiter = ',', required = true)]
        tags: Vec<String>,
    },
    /// List the project's valid work item types (and their states).
    ListWorkItemTypes,
    /// List the members of the configured team.
    ListTeamMembers,
    /// Search the org's users by name or email (people picker).
    SearchUsers {
        /// Text to search: display name or email (or part of them).
        query: String,
    },
    /// Install the Claude Code skill into ./.claude/skills/ (overwrites).
    Skill,
}
