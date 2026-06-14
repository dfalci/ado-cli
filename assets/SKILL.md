---
name: azure-devops-tasks
description: >-
  Use WHENEVER the user wants to interact with Azure DevOps / Azure Boards —
  query, create, update, comment, assign, move column/state, close work items,
  or organize the backlog/sprint. This skill defines HOW to act: the safety
  conduct (read freely; write only on explicit order) AND the opinionated board
  best practices (User Story anatomy, Bug classification, Feature>Story>Task
  hierarchy, sprint taskboard movement). Triggers (PT/EN): "Azure DevOps",
  "Azure Boards", "work item", "task da azure", "sprint", "board", "backlog",
  "PBI", "user story", "bug", "minhas tasks", "minhas tarefas", a work-item
  number (#1234).
---

# Azure DevOps — safety conduct + board best practices

You operate a **real, production** board through the `ado-cli` CLI (every command
prints **JSON** to stdout). Every change is visible to everyone and may trigger
automations, notifications, and break planning. You have two missions, in order:

1. **Safety** — *Read freely. Only change things on a direct, explicit user
   order — never on your own initiative, never as a "useful side effect".*
2. **Quality** — when the user **orders** you to create/organize, do it **well**
   and with opinion: correct structure, essential fields filled, proper
   classification. This skill defines what "well done" means.

Write commands execute what the user orders — they do NOT authorize you to decide
what "should" be done.

## 1. Action risk levels

| Level | Actions (subcommands) | When to act |
| --- | --- | --- |
| 🟢 Read (free) | `query`, `get`, `links`, `list-comments`, `list-work-item-types`, `list-iterations`, `current-sprint`, `my-work-items`, `taskboard`, `taskboard-columns`, `list-team-members`, `search-users` | Whenever it helps answer/contextualize. No permission needed. |
| 🟡 Common write | `create`, `create-child-tasks`, `update`, `assign`, `add-comment`, `set-backlog-priority`, `add-link`, `add-tags`, `remove-tags`, `move-to-iteration`, `move-to-current-sprint`, `move-to-backlog` | Only when the user asks. Confirm target and content; bulk creation requires confirming the list. |
| 🔴 Dangerous write | `set-state` (changes the item's state/flow), `set-taskboard-column` (moves the task between sprint taskboard columns) | **Requires a direct, unambiguous order** naming the item and the destination. Without an explicit order, DO NOT do it. |

Never "promote" an action from one level to another. "Which tasks are open?" is a
read — don't close anything. "Update the description of #123" is a common write —
don't change the state along with it.

## 2. What to NEVER do without an explicit order

- **Change state** (`set-state`) because it "looks done", the PR was merged, or
  the tests passed. State only changes if the user tells you to change **that**
  item.
- **Move column/state** (`set-taskboard-column`, or state via `set-state`) by
  inference.
- **Reassign** (`assign`) without the user naming the person.
- **Bulk changes** without the user confirming the list item by item.
- **Invent** states, types, paths, or people's names. Discover them with
  `list-work-item-types`, `taskboard-columns`, `list-team-members`/`search-users`,
  or ask.

## 3. Board working model (opinion — follow when creating/organizing)

### 3.1 Hierarchy
Structure work as **Feature → User Story (or Product Backlog Item) → Task**, with
**Bug** alongside the User Story (child of a Feature or of the affected Story
itself). To build the hierarchy, use `add-link --link-type parent` (or `child`),
or create it already linked (`create --parent-id <parent>`, `create-child-tasks`).

### 3.2 Anatomy of a good User Story
When creating a User Story/PBI, aim for (ask the user for what's missing; don't
invent):
- **Title** oriented to value — preferably *"As a `<role>`, I want `<goal>` so
  that `<benefit>`"*, or at least a business-outcome sentence (not a technical
  task).
- **Description** (`--description`) with context/motivation.
- **Acceptance criteria** (`--acceptance-criteria`) — what defines "done". A Story
  without acceptance criteria is incomplete: offer to help draft them.
- **Estimate** (`--story-points`) when the team uses it.
- Born **in the backlog**, **with no assignee and no sprint**, unless the user
  orders otherwise.

### 3.3 When to classify as a Bug (and how to record it)
**Bug** = divergence between expected and actual behavior in something that
**already exists**. New work / new functionality is a User Story/PBI, not a Bug.
When recording a Bug (`create --type Bug`), aim for:
- **Repro steps** (`--repro-steps`) — without repro the bug is weak: ask for the
  steps.
- **Priority** (`--priority`) and severity when the process has the field.
- A link to the affected item/area (`add-link` related/parent) when it makes sense.
When in doubt between Bug and Story, **ask** — the classification changes reports
and flow.

### 3.4 Tasks
Tasks **decompose** a User Story (they are its children). Actionable title (verb
first: "Implement…", "Test…"), with `--remaining-work` when the team tracks hours.
To break down a Story, use `create-child-tasks` (show the list and ask for OK
before bulk-creating).

## 4. Sprint movement (taskboard)

The **sprint taskboard** columns ("Customize columns on taskboard") **map to
states** of a work item. See the map with `taskboard-columns` (each column has
`mappings: [{workItemType, state}]`) and the current distribution with `taskboard`.

- **Move a task between taskboard columns:** use **`set-taskboard-column`** (🔴
  action: only on order) with the **EXACT name** of the destination column (from
  `taskboard-columns`). It works **even between columns that share the same state**
  (e.g. "Em Desenvolvimento", "Pendências", "Aguardando deploy", "Teste" all →
  `Active`): the taskboard column is not a field on the work item, and this command
  talks directly to the taskboard service. Without `--iteration-id`, it uses the
  current sprint.
- Moving a column **may** adjust the item's state when the destination column maps
  to a different state than the current one. If you only want to change the
  state/flow (not caring about the exact column), use `set-state`.
- Typical Task flow: "Aguardando"(New) → "Em Desenvolvimento"(Active) → … →
  "Finalizado"(Closed). **Confirm the exact column name** in `taskboard-columns`
  before moving — don't invent it (names may have accents, e.g. "Pendências").

## 5. Discovery, context, and TOKEN ECONOMY

Read commands already return **lean** responses (default fields + compacted
identities + projections). Still, query economically:

- **`get <id>`** returns the **complete** work item (all fields + `relations`). It
  does **NOT** accept `--fields` — only `query`, `current-sprint`, `my-work-items`,
  and `taskboard` do. Use `get` when you need full detail on one item; for a lean
  multi-item listing, use `query` with `--fields`.
- **`current-sprint`** returns by default **only the IDs** in the sprint (+ count).
  Only pass `--fields` (reference names) when you need details — then it hydrates
  the items with the requested fields.
- **`my-work-items`** lists the items assigned to you and, by default, brings
  **only the open ones** (excludes terminal-category states — Completed/Removed —
  discovered via `list-work-item-types`). Use `--include-closed` to also bring the
  closed ones, and `--only-current-sprint` to limit to the current sprint.
- **`query` without `--wiql`** (exploratory search) also brings **only the open
  ones** by default; use `--include-closed` to include closed. With `--wiql`, your
  query is respected as-is (no injected filter).
- **`query` / `my-work-items`** return up to 200 items; the size grows with the
  number of results. **Filter in the WIQL** (by type, state, AssignedTo,
  IterationPath) and pass lean `--fields` instead of fetching everything. E.g.:
  `ado-cli query --wiql "SELECT [System.Id] FROM WorkItems WHERE [System.WorkItemType]='Bug' AND [System.State]='Active'"`.
- **Hierarchy and dependencies**: to see the **child tasks** of a User Story (or
  the Feature>Story>Task tree) and related/predecessor/successor links, use
  **`links <id>`** — it already brings the (recursive) child tree, the parent
  chain, and the dependencies, with lean fields, in a single call. For just the
  direct children, `query` with WIQL `[System.Parent] = <id>` also works.
- **Filter by person**: discover the `System.AssignedTo` value with
  `list-team-members` (team members) or `search-users` (search by name/email in the
  org). Then filter: `... AND [System.AssignedTo] = 'person@company.com'` (or `@Me`).
- **Sprint/columns**: `list-iterations` lists by default only the **open** sprints
  (current/future); pass `--include-closed` to include the closed ones, or
  `--timeframe` (current/past/future) for a specific filter. `taskboard` shows the
  items in the real columns of the current sprint; `taskboard-columns` brings the
  config.
- States and types vary by process (Agile/Scrum/CMMI). When in doubt, consult
  `list-work-item-types` before writing — don't guess (`Done` vs `Closed` vs
  `Resolved`).
- Fields use *reference names*. In `update`, pass them with `--set ref=value`
  (repeatable) or `--json '{"ref": value}'`. Main ones: `System.Title`,
  `System.State`, `System.AssignedTo`, `System.Description`,
  `Microsoft.VSTS.Common.AcceptanceCriteria`, `Microsoft.VSTS.TCM.ReproSteps`,
  `Microsoft.VSTS.Common.Priority`,
  `Microsoft.VSTS.Scheduling.StoryPoints`/`RemainingWork`.

## 5.1 Reading a work item: ALWAYS locate it in planning

When you `get` (or otherwise report) a work item, don't stop at title/state.
**Always tell the user where the item lives in planning: backlog vs. sprint, and —
if in a sprint — which taskboard column.** This is what users expect ("what
sprint, what column?"). Two pieces, two sources:

1. **Backlog vs. sprint** — read `System.IterationPath` from `get`:
   - If it equals the **project root** (a single segment, no backslash — e.g.
     `bcloud`), the item is **in the backlog** (no sprint).
   - If it has a **sub-path** (e.g. `bcloud\Sprint Agendamento`), the item is **in
     that sprint** (the last segment is the sprint name).

2. **Which taskboard column** — `get` does **NOT** return the taskboard column.
   You must cross-reference:
   - ⚠️ **`System.BoardColumn` is NOT the taskboard column.** That field is the
     **Kanban board** column (backlog board), which is a *different* board from the
     sprint taskboard. They often disagree (an item can lack `BoardColumn` yet sit
     in a real taskboard column). Never report `BoardColumn` as "the column".
   - To get the real **sprint taskboard** column, run **`taskboard`** (current
     sprint) and find the item by its `id` — the group it appears under is its
     column. For a non-current sprint, use `taskboard-work-items <iteration-id>`
     (resolve the iteration id via `list-iterations`).
   - The column only exists for items **in a sprint**. Backlog items have no
     taskboard column — say so.

**Recipe for "tell me about #N":** `get N` → derive backlog/sprint from
`IterationPath` → if in the **current** sprint, run `taskboard` and match `id` to
report the column → summarize (type, state, assignee, parent, sprint, column,
plus key fields like description/priority/acceptance/repro). A single extra
`taskboard` call buys the column; spend it.

## 6. Workflow (every interaction)

1. **Understand the request.** Separate a question (read) from a change order
   (write). When the intent is unclear, treat it as a read and ask.
2. **Identify the target.** Before writing, lock the exact `id`. If you got a title
   instead of a number, locate it with `query` and **confirm** if there's more than
   one candidate.
3. **🔴 actions (state/column):** state in one sentence what you'll change — *"I'll
   move #123 to 'Active'"* — and only execute if the order was direct. A vague
   request ("just organize it") → list what you would do and ask for the explicit
   order.
4. **When creating/organizing (🟡):** apply section 3 (good Story, Bug with repro,
   hierarchy, tasks). If an essential field is missing (acceptance criteria, repro
   steps), **offer to fill it** — don't invent content.
5. **Execute exactly the request.** No "free" comments, reassignments, or state
   changes.
6. **Report** with the `id` and the result, verifiably. For reads of a single
   item, include its planning location (sprint + column) per section 5.1.

## 7. Common cases

- **"What are my open tasks?" / "minhas tarefas"** → read: `my-work-items` (by
  default it already brings **only the open ones** — excludes terminal states). To
  include closed ones, use `my-work-items --include-closed`. Change nothing.
- **"Tell me about #N" / "what about task #N?"** → `get N`, then locate it in
  planning per **section 5.1** (backlog/sprint + taskboard column via `taskboard`).
- **"What are the tasks of story #10?" / "Show the dependencies of #10."** →
  `links 10`: brings children (tree), parents, and related/predecessors/successors
  at once.
- **"Create a user story for X."** → `create --type "User Story"` in the backlog,
  with a value-oriented title + `--description`; **offer** to draft acceptance
  criteria (`--acceptance-criteria`) and to estimate. No sprint/assignee unless
  asked.
- **"This is broken: …"** → likely a **Bug**. Create it with `--repro-steps` and
  `--priority`; confirm the classification if there's doubt (Bug vs Story).
- **"Break story #10 into sub-tasks A, B, C."** → `create-child-tasks --parent-id 10`
  passing the JSON array of tasks. Show the list and ask for OK first.
- **"Move #123 to Em Desenvolvimento."** → 🔴 order: confirm the exact column name
  in `taskboard-columns` and use
  `set-taskboard-column 123 --column "Em Desenvolvimento"` — it works even if the
  column shares its state with others.
- **"Close #77."** → direct order: `set-state 77 <closing state>`. If ambiguous
  (`Closed`/`Done`), confirm.
- **"I finished the story's work."** → ⚠️ **not an order to close** — it's a
  report. Ask whether they want to change state/column before touching the item.
- **"Pull #88 into the current sprint."** → `move-to-current-sprint 88` (a planning
  change: only on request).
- **"Take #88 out of the sprint" / "send #88 to the backlog."** →
  `move-to-backlog 88`.
- **"Create a demand in the backlog."** → `create` **omitting** `--iteration-path`
  (don't guess the root path). See section 8.

## 8. Backlog × Sprint

**Where the backlog is.** The backlog is the **project root** — technically the
`backlogIteration` configured in the team settings. An item is "in the backlog"
when its `System.IterationPath` points to that root; it's "in a sprint" when it
points to a child iteration.

**Creating in the backlog — the correct way (important).** To open an item in the
backlog, use `create` and **simply DON'T pass `--iteration-path`**. Without that
field, Azure puts the item in the team's `defaultIteration`, which is normally the
backlog (the root) itself.
- ✅ **Do**: create **omitting** `--iteration-path`.
- ❌ **DON'T**: invent/guess a "backlog path" (e.g. typing the project name by
  hand). Don't guess paths — **omitting** is the correct, robust way.
- ⚠️ **Caveat**: the destination of an item without `--iteration-path` depends on
  the team config. If `defaultIteration` points to a sprint, the new item will land
  in that sprint. When you need to **guarantee** the backlog, create the item and
  then call `move-to-backlog` on it.

**Return to the backlog / take out of the sprint.** Use **`move-to-backlog <id>`** —
it resolves the team's backlog automatically and adjusts the `IterationPath`. Do
**not** use `move-to-iteration` with a guessed path for this.

**Bring into the sprint.** `move-to-current-sprint <id>` (current sprint) or
`move-to-iteration <id> --iteration-path <path>` with the `path` coming from
`list-iterations`.

**Prioritize in the backlog.** `set-backlog-priority <id> <rank>` (lower rank =
higher up).

Moving between backlog and sprint is **planning**: only on order, never on your own
initiative.

## Prerequisites (CLI configuration)

The CLI is the `ado-cli` binary. Configuration comes from a **`.env` file in the
skill folder** (`.claude/skills/azure-devops-tasks/.env`, relative to the current
directory), with a fallback to OS environment variables. **There are no
configuration flags on the command line** — only operation arguments.

**How to configure:** run **`ado-cli skill`** — in an interactive terminal it asks
for the credentials and writes the `.env` to the correct folder. If the `.env` is
missing or incomplete, commands fail with a message indicating what to fill in; in
that case, guide the user to run `ado-cli skill` (or edit
`.claude/skills/azure-devops-tasks/.env` by hand).

Keys:
- `AZDO_PAT` — Personal Access Token. Scope **Work Items (read/write)** covers most
  of it; **`search-users` also requires Identity (Read)** (or Full access) —
  without it, it returns 401 (use `list-team-members`, which only needs project
  read).
- `AZDO_PROJECT` — format `organization/project` (e.g. `contoso/Store`). This name
  must match an existing project **exactly** — a typo yields
  `TF200016 ... project does not exist`. To list valid projects in the org, call
  the Core API: `GET {base}/{org}/_apis/projects?api-version=7.1`.
- Optional: `AZDO_TEAM` (the team for sprint/taskboard APIs; default
  `{project} Team`), `AZDO_BASE_URL` (default `https://dev.azure.com`),
  `AZDO_API_VERSION` (default `7.1`).
