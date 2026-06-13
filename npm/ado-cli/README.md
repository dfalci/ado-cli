# @dfalci/ado-cli

CLI para manipular **work items de um board do Azure DevOps** pela linha de
comando (saída sempre em **JSON**). Wrapper npm de um binário nativo escrito em
Rust — o binário da sua plataforma é instalado automaticamente via
`optionalDependencies`.

## Instalação

```bash
npm install -g @dfalci/ado-cli
```

Plataformas suportadas: Linux x64, macOS x64, macOS arm64, Windows x64.

## Configuração

A configuração vem de um arquivo **`.env` no diretório atual** (com fallback para
variáveis de ambiente do SO). **Não há flags de configuração.**

```
AZDO_PAT=<seu-pat>
AZDO_PROJECT=organizacao/projeto
```

Opcionais: `AZDO_TEAM` (default `{projeto} Team`), `AZDO_BASE_URL`
(default `https://dev.azure.com`), `AZDO_API_VERSION` (default `7.1`).

## Uso

```bash
ado-cli --help
ado-cli my-work-items          # suas tarefas abertas
ado-cli query                  # itens recentes abertos
ado-cli get 123
```

Veja o repositório para a lista completa de comandos e exemplos:
https://github.com/dfalci/ado-cli

## Licença

MIT.
