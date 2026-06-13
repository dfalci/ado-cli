---
name: azure-devops-tasks
description: >-
  Use SEMPRE que o usuário falar em interagir com o Azure DevOps / Azure Boards —
  consultar, criar, atualizar, comentar, atribuir, mover de coluna/estado, fechar
  work items, ou organizar backlog/sprint. Esta skill define COMO agir: a conduta
  de segurança (leitura livre; escrita só sob ordem) E as boas práticas opinativas
  de organização do board (anatomia de User Story, classificação de Bug, hierarquia
  Feature>Story>Task, movimentação no taskboard da sprint). Gatilhos: "Azure DevOps",
  "Azure Boards", "work item", "task da azure", "sprint", "board", "backlog", "PBI",
  "user story", "bug", número de work item (#1234).
---

# Azure DevOps — conduta de segurança + boas práticas do board

Você opera um board **real, de produção**, através da CLI `ado-cli` (cada comando
imprime **JSON** no stdout). Cada alteração é visível para todos e pode disparar
automações, notificações e quebrar planejamento. Você tem duas missões, nesta ordem:

1. **Segurança** — *Leia à vontade. Só altere sob ordem direta e explícita do
   usuário — nunca por iniciativa própria, nunca como "efeito colateral útil".*
2. **Qualidade** — quando o usuário **mandar** criar/organizar, faça **bem feito**
   e com opinião: estrutura correta, campos essenciais preenchidos, classificação
   adequada. Esta skill diz qual é o "bem feito".

Os comandos de escrita executam o que o usuário mandar — eles não te autorizam a
decidir o que "deveria" ser feito.

## 1. Níveis de risco das ações

| Nível | Ações (subcomandos) | Quando agir |
| --- | --- | --- |
| 🟢 Leitura (livre) | `query`, `get`, `links`, `list-comments`, `list-work-item-types`, `list-iterations`, `current-sprint`, `my-work-items`, `taskboard`, `taskboard-columns`, `list-team-members`, `search-users` | Sempre que ajudar a responder/contextualizar. Não precisa pedir permissão. |
| 🟡 Escrita comum | `create`, `create-child-tasks`, `update`, `assign`, `add-comment`, `set-backlog-priority`, `add-link`, `add-tags`, `remove-tags`, `move-to-iteration`, `move-to-current-sprint`, `move-to-backlog` | Só quando o usuário pedir. Confirme alvo e conteúdo; criação em lote exige confirmação da lista. |
| 🔴 Escrita perigosa | `set-state` (muda o estado/fluxo do item), `set-taskboard-column` (move a task entre colunas do taskboard da sprint) | **Exige ordem direta e inequívoca**, identificando o item e o destino. Sem ordem explícita, NÃO faça. |

Nunca "promova" uma ação de um nível para outro. "Quais tasks estão abertas?" é
leitura — não feche nada. "Atualize a descrição da #123" é escrita comum — não
mude o estado junto.

## 2. O que NUNCA fazer sem ordem expressa

- **Mudar estado** (`set-state`) porque "parece concluído", o PR foi mergeado ou
  os testes passaram. Estado só muda se o usuário disser para mudar **aquele** item.
- **Mover de coluna/estado** (`set-taskboard-column`, ou estado via `set-state`)
  por inferência.
- **Reatribuir** (`assign`) sem o usuário nomear a pessoa.
- **Alterações em massa** sem o usuário confirmar a lista item por item.
- **Inventar** estados, tipos, paths ou nomes de pessoa. Descubra com
  `list-work-item-types`, `taskboard-columns`, `list-team-members`/`search-users`,
  ou pergunte.

## 3. Modelo de trabalho do board (opinião — siga ao criar/organizar)

### 3.1 Hierarquia
Estruture o trabalho como **Feature → User Story (ou Product Backlog Item) → Task**,
com **Bug** ao lado da User Story (filho de Feature ou da própria Story afetada).
Para montar a hierarquia, use `add-link --link-type parent` (ou `child`), ou crie
já vinculado (`create --parent-id <pai>`, `create-child-tasks`).

### 3.2 Anatomia de uma boa User Story
Ao criar uma User Story/PBI, busque (peça ao usuário o que faltar; não invente):
- **Título** orientado a valor — preferencialmente *"Como `<papel>`, quero
  `<objetivo>` para `<benefício>`"*, ou ao menos uma frase de resultado de negócio
  (não uma tarefa técnica).
- **Descrição** (`--description`) com contexto/motivação.
- **Critérios de aceite** (`--acceptance-criteria`) — o que define "pronto". Uma
  Story sem critérios de aceite é incompleta: ofereça ajudar a redigi-los.
- **Estimativa** (`--story-points`) quando o time usar.
- Nasce **no backlog**, **sem responsável e sem sprint**, salvo ordem do usuário.

### 3.3 Quando classificar como Bug (e como registrar)
**Bug** = divergência entre o comportamento esperado e o real em algo que **já
existe**. Trabalho novo/funcionalidade nova é User Story/PBI, não Bug.
Ao registrar um Bug (`create --type Bug`), busque:
- **Passos de reprodução** (`--repro-steps`) — sem repro, o bug é fraco: peça os passos.
- **Prioridade** (`--priority`) e severidade quando o processo tiver o campo.
- Vínculo ao item/área afetada (`add-link` related/parent) quando fizer sentido.
Na dúvida entre Bug e Story, **pergunte** — a classificação muda relatórios e fluxo.

### 3.4 Tasks
Tasks **decompõem** uma User Story (são filhas dela). Título acionável (verbo no
início: "Implementar…", "Testar…"), com `--remaining-work` quando o time controlar
horas. Para quebrar uma Story, use `create-child-tasks` (mostre a lista e peça OK
antes de criar em lote).

## 4. Movimentação na sprint (taskboard)

As colunas do **taskboard de sprint** ("Customize columns on taskboard") **mapeiam
estados** de work item. Veja o mapa com `taskboard-columns` (cada coluna tem
`mappings: [{workItemType, state}]`) e a distribuição atual com `taskboard`.

- **Mover uma task entre colunas do taskboard:** use **`set-taskboard-column`**
  (ação 🔴: só sob ordem) com o **nome EXATO** da coluna destino (de
  `taskboard-columns`). Funciona **inclusive entre colunas que compartilham o
  mesmo estado** (ex.: "Em Desenvolvimento", "Pendências", "Aguardando deploy",
  "Teste" todas → `Active`): a coluna do taskboard não é um campo do work item, e
  esse comando fala direto com o serviço de taskboard. Sem `--iteration-id`, usa a
  sprint atual.
- Mover de coluna **pode** ajustar o estado do item quando a coluna destino mapeia
  um estado diferente do atual. Se você só quer mudar o estado/fluxo (sem se
  importar com a coluna exata), use `set-state`.
- Fluxo típico de uma Task: "Aguardando"(New) → "Em Desenvolvimento"(Active) → … →
  "Finalizado"(Closed). **Confirme o nome exato** da coluna em `taskboard-columns`
  antes de mover — não invente (nomes podem ter acento, ex.: "Pendências").

## 5. Descoberta, contexto e ECONOMIA de tokens

Os comandos de leitura já devolvem respostas **enxutas** (campos default + identidades
compactadas + projeções). Ainda assim, consulte de forma econômica:

- **`current-sprint`** devolve por padrão **apenas os IDs** da sprint (+ contagem).
  Só passe `--fields` (reference names) quando precisar dos detalhes — aí ele
  hidrata os itens com os campos pedidos.
- **`my-work-items`** lista os itens atribuídos a você e, por padrão, traz **só os
  abertos** (exclui os estados de categoria terminal — Completed/Removed —
  descobertos via `list-work-item-types`). Use `--include-closed` para trazer
  também os fechados, e `--only-current-sprint` para limitar à sprint corrente.
- **`query` sem `--wiql`** (busca exploratória) também traz por padrão **só os
  abertos**; use `--include-closed` para incluir os fechados. Com `--wiql`, sua
  consulta é respeitada como está (sem filtro injetado).
- **`query` / `my-work-items`** retornam até 200 itens; o tamanho cresce com o nº
  de resultados. **Filtre na WIQL** (por tipo, estado, AssignedTo, IterationPath) e
  passe `--fields` enxutos em vez de trazer tudo. Ex.:
  `ado-cli query --wiql "SELECT [System.Id] FROM WorkItems WHERE [System.WorkItemType]='Bug' AND [System.State]='Active'"`.
- **Hierarquia e dependências**: para ver as **tarefas filhas** de uma User Story
  (ou a árvore Feature>Story>Task) e os vínculos related/predecessor/successor, use
  **`links <id>`** — ele já traz a árvore de filhos (recursiva), a cadeia de pais e
  as dependências, com campos enxutos, numa só chamada. Para só os filhos diretos,
  `query` com WIQL `[System.Parent] = <id>` também serve.
- **Filtrar por pessoa**: descubra o valor de `System.AssignedTo` com
  `list-team-members` (membros do time) ou `search-users` (busca por nome/e-mail na
  org). Depois filtre: `... AND [System.AssignedTo] = 'pessoa@empresa.com'` (ou `@Me`).
- **Sprint/colunas**: `list-iterations` lista por padrão só as sprints **abertas**
  (current/future); passe `--include-closed` para incluir as encerradas, ou
  `--timeframe` (current/past/future) para um filtro específico. `taskboard` mostra
  os itens nas colunas reais da sprint atual; `taskboard-columns` traz a config.
- Estados e tipos variam por processo (Agile/Scrum/CMMI). Na dúvida, consulte
  `list-work-item-types` antes de escrever — não chute (`Done` vs `Closed` vs `Resolved`).
- Campos usam *reference names*. No `update`, informe-os com `--set ref=valor`
  (repetível) ou `--json '{"ref": valor}'`. Principais: `System.Title`,
  `System.State`, `System.AssignedTo`, `System.Description`,
  `Microsoft.VSTS.Common.AcceptanceCriteria`, `Microsoft.VSTS.TCM.ReproSteps`,
  `Microsoft.VSTS.Common.Priority`, `Microsoft.VSTS.Scheduling.StoryPoints`/`RemainingWork`.

## 6. Fluxo de trabalho (toda interação)

1. **Entenda o pedido.** Separe pergunta (leitura) de ordem de alteração (escrita).
   Na dúvida sobre a intenção, trate como leitura e pergunte.
2. **Identifique o alvo.** Antes de escrever, fixe o `id` exato. Se veio título em
   vez de número, localize com `query` e **confirme** se houver mais de um candidato.
3. **Ações 🔴 (estado/coluna):** declare numa frase o que vai mudar — *"vou mover a
   #123 para 'Active'"* — e só execute se a ordem foi direta. Pedido vago
   ("organiza aí") → liste o que faria e peça a ordem explícita.
4. **Ao criar/organizar (🟡):** aplique a seção 3 (boa Story, Bug com repro,
   hierarquia, tasks). Se faltar campo essencial (critérios de aceite, repro
   steps), **ofereça preencher** — não invente conteúdo.
5. **Execute exatamente o pedido.** Nada de comentários, reatribuições ou mudanças
   de estado "de brinde".
6. **Reporte** com o `id` e o resultado, de forma verificável.

## 7. Casos comuns

- **"Quais minhas tasks abertas?"** → leitura: `my-work-items` (por padrão já traz
  **só os abertos** — exclui estados terminais). Para incluir os fechados, use
  `my-work-items --include-closed`. Não altere nada.
- **"Quais as tarefas da story #10?" / "Mostra as dependências da #10."** →
  `links 10`: traz filhas (árvore), pais e related/predecessores/sucessores de uma vez.
- **"Cria uma user story para X."** → `create --type "User Story"` no backlog, com
  título de valor + `--description`; **ofereça** redigir critérios de aceite
  (`--acceptance-criteria`) e estimar. Sem sprint/responsável salvo pedido.
- **"Isso aqui está quebrado: …"** → provável **Bug**. Crie com `--repro-steps` e
  `--priority`; confirme classificação se houver dúvida (Bug vs Story).
- **"Quebra a story #10 em sub-tasks A, B, C."** → `create-child-tasks --parent-id 10`
  passando o array JSON de tasks. Mostre a lista e peça OK antes.
- **"Move a #123 para Em Desenvolvimento."** → ordem 🔴: confirme o nome exato da
  coluna em `taskboard-columns` e use
  `set-taskboard-column 123 --column "Em Desenvolvimento"` — funciona mesmo se a
  coluna compartilhar estado com outras.
- **"Fecha a #77."** → ordem direta: `set-state 77 <estado de fechamento>`. Se
  ambíguo (`Closed`/`Done`), confirme.
- **"Terminei o trabalho da story."** → ⚠️ **não é ordem de fechar** — é relato.
  Pergunte se quer mudar estado/coluna antes de tocar no item.
- **"Puxa a #88 para a sprint atual."** → `move-to-current-sprint 88` (mudança de
  planejamento: só sob pedido).
- **"Tira a #88 da sprint" / "joga a #88 pro backlog."** → `move-to-backlog 88`.
- **"Cria uma demanda no backlog."** → `create` **omitindo** `--iteration-path`
  (não chute o path da raiz). Veja a seção 8.

## 8. Backlog × Sprint

**Onde fica o backlog.** O backlog é a **raiz do projeto** — tecnicamente o
`backlogIteration` configurado nas opções do time. Um item está "no backlog"
quando seu `System.IterationPath` aponta para essa raiz; está "numa sprint" quando
aponta para uma iteração filha.

**Criar no backlog — forma correta (importante).** Para abrir um item no backlog,
use `create` e **simplesmente NÃO informe `--iteration-path`**. Sem esse campo, o
Azure coloca o item no `defaultIteration` do time, que normalmente é o próprio
backlog (a raiz).
- ✅ **Faça**: criar **omitindo** `--iteration-path`.
- ❌ **NÃO faça**: inventar/chutar um path "de backlog" (ex.: digitar o nome do
  projeto na mão). Não chute paths — **omitir** é o caminho certo e robusto.
- ⚠️ **Ressalva**: o destino de um item sem `--iteration-path` depende da config do
  time. Se o `defaultIteration` apontar para uma sprint, o item novo cairá nessa
  sprint. Quando precisar **garantir** o backlog, crie o item e em seguida chame
  `move-to-backlog` nele.

**Devolver ao backlog / tirar da sprint.** Use **`move-to-backlog <id>`** — ele
resolve o backlog do time automaticamente e ajusta o `IterationPath`. **Não** use
`move-to-iteration` com um path adivinhado para isso.

**Trazer para a sprint.** `move-to-current-sprint <id>` (sprint corrente) ou
`move-to-iteration <id> --iteration-path <path>` com o `path` vindo de `list-iterations`.

**Priorizar no backlog.** `set-backlog-priority <id> <rank>` (menor rank = mais acima).

Mover entre backlog e sprint é **planejamento**: só sob ordem, nunca por iniciativa.

## Pré-requisitos (configuração da CLI)

A CLI é o binário `ado-cli`. Toda a configuração vem de um arquivo **`.env` no
diretório atual** (com fallback para variáveis de ambiente do SO). **Não há flags
de configuração na linha de comando** — só argumentos das operações.

- `AZDO_PAT` — Personal Access Token. Escopo **Work Items (read/write)** cobre a
  maior parte; **`search-users` requer também Identity (Read)** (ou Full access) —
  sem isso ela retorna 401 (use `list-team-members`, que precisa só de leitura de projeto).
- `AZDO_PROJECT` — formato `organizacao/projeto` (ex.: `contoso/Loja`).
- Opcionais: `AZDO_TEAM` (time das APIs de sprint/taskboard; default `{projeto} Team`),
  `AZDO_BASE_URL` (default `https://dev.azure.com`), `AZDO_API_VERSION` (default `7.1`).

Exemplo de `.env`:

```
AZDO_PAT=<seu-pat>
AZDO_PROJECT=contoso/Loja
```
