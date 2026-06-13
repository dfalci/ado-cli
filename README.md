# ado-cli

CLI local (Rust) que dá acesso às **tasks (work items) de um board do Azure
DevOps** pela linha de comando: listar/consultar, criar, atualizar, mudar de
estado, atribuir, mover de coluna no taskboard, vincular itens (hierarquia do
backlog), reordenar o backlog, navegar o contexto de sprint, decompor itens em
sub-tasks, gerenciar tags e comentários.

Toda operação imprime **JSON** no stdout. É a contraparte de linha de comando do
servidor MCP `mcp-ado` (que segue existindo, independente).

## Build

```bash
cargo build --release
# binário em target/release/ado-cli
```

## Configuração

A configuração vem de um arquivo **`.env` no diretório atual** (formato
`CHAVE=valor`). Para cada chave ausente no arquivo, cai-se para a variável de
ambiente do SO de mesmo nome. **Não há flags de configuração na CLI** — a linha
de comando recebe apenas os argumentos das operações.

| Variável            | Obrigatória | Default                  | Descrição                                       |
| ------------------- | ----------- | ------------------------ | ----------------------------------------------- |
| `AZDO_PAT`          | sim         | —                        | Personal Access Token (escopo Work Items r/w).  |
| `AZDO_PROJECT`      | sim         | —                        | No formato `organizacao/projeto`.               |
| `AZDO_TEAM`         | não         | `{projeto} Team`         | Time usado nas APIs de sprint/iteração.         |
| `AZDO_BASE_URL`     | não         | `https://dev.azure.com`  | Útil para Azure DevOps Server on-prem.          |
| `AZDO_API_VERSION`  | não         | `7.1`                    | Versão da API REST.                             |

Exemplo de `.env`:

```
AZDO_PAT=<seu-pat>
AZDO_PROJECT=contoso/Loja
```

## Uso

```bash
ado-cli <comando> [args]
# saída sempre em JSON no stdout

ado-cli --help          # lista todos os comandos
ado-cli <comando> --help
```

### Exemplos

```bash
# Leitura
ado-cli query                               # sem WIQL: só os mais recentes ABERTOS
ado-cli query --include-closed              # sem WIQL: inclui os fechados
ado-cli query --wiql "SELECT [System.Id] FROM WorkItems WHERE [System.WorkItemType]='Bug' AND [System.State]='Active'"
ado-cli get 123
ado-cli links 10
ado-cli current-sprint --fields System.Id,System.Title,System.State
ado-cli taskboard
ado-cli my-work-items                       # por padrão, só os abertos
ado-cli my-work-items --include-closed      # inclui os fechados
ado-cli my-work-items --only-current-sprint

# Escrita
ado-cli create --type Bug --title "Erro no checkout" --repro-steps "1. ..." --priority 1
ado-cli update 123 --set System.Title="Novo título" --set Microsoft.VSTS.Common.Priority=2
ado-cli update 123 --json '{"System.Title":"Novo título","Microsoft.VSTS.Common.Priority":2}'
ado-cli assign 123 "fulano@empresa.com"
ado-cli add-link 10 42 --link-type child
ado-cli add-comment 123 "Comentário"

# Escrita 🔴 (estado/coluna): só sob ordem explícita
ado-cli set-state 77 Closed
ado-cli set-taskboard-column 123 --column "Em Desenvolvimento"

# Decompor um pai em sub-tasks (array JSON via --json ou stdin)
ado-cli create-child-tasks --parent-id 10 --json '[{"title":"Implementar API"},{"title":"Testar"}]'
echo '[{"title":"A"},{"title":"B"}]' | ado-cli create-child-tasks --parent-id 10
```

Comandos com estrutura complexa (`update`, `create-child-tasks`) aceitam **JSON**
via flag (`--json`) ou pelo **stdin** quando a flag é omitida.

## Skill do Claude Code

O binário pode instalar uma skill que ensina o agente a usar esta CLI:

```bash
cd /seu/projeto
ado-cli skill
# cria ./.claude/skills/azure-devops-tasks/SKILL.md (sobrescreve se existir)
```

## Desenvolvimento

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

## Licença

MIT.
