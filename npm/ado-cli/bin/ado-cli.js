#!/usr/bin/env node
"use strict";

// Launcher do @dfalci/ado-cli: resolve o binário nativo da plataforma atual
// (instalado via optionalDependencies) e o executa repassando argumentos,
// stdin/stdout/stderr e o código de saída. A saída JSON do binário passa direta.

const { spawnSync } = require("child_process");
const path = require("path");
const fs = require("fs");

// Mapa plataforma+arquitetura do Node -> subpacote com o binário.
const PACKAGES = {
  "linux-x64": "@dfalci/ado-cli-linux-x64",
  "darwin-x64": "@dfalci/ado-cli-darwin-x64",
  "darwin-arm64": "@dfalci/ado-cli-darwin-arm64",
  "win32-x64": "@dfalci/ado-cli-win32-x64",
};

function resolveBinary() {
  const key = `${process.platform}-${process.arch}`;
  const pkg = PACKAGES[key];
  if (!pkg) {
    throw new Error(
      `Plataforma não suportada: ${key}. Suportadas: ${Object.keys(PACKAGES).join(", ")}.`
    );
  }
  const binName = process.platform === "win32" ? "ado-cli.exe" : "ado-cli";
  // Resolve via o package.json do subpacote (robusto p/ arquivos sem extensão).
  let pkgDir;
  try {
    pkgDir = path.dirname(require.resolve(`${pkg}/package.json`));
  } catch (_) {
    throw new Error(
      `Pacote ${pkg} não encontrado. Reinstale com a optionalDependency da sua plataforma habilitada.`
    );
  }
  const binPath = path.join(pkgDir, "bin", binName);
  if (!fs.existsSync(binPath)) {
    throw new Error(`Binário não encontrado em ${binPath}.`);
  }
  return binPath;
}

let binary;
try {
  binary = resolveBinary();
} catch (err) {
  console.error(err.message);
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}
process.exit(result.status === null ? 1 : result.status);
