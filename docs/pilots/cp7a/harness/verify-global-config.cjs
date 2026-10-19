// Reconstruct the benchmark-time OpenCode global config from the archived original
// backup plus the two edits recorded in the session history, then verify it is
// byte-identical to the archived benchmark config and to the frozen SHA-256
// 1E9671EC... recorded in method/environment.json before measurement.
// Run: node harness/verify-global-config.cjs   (from docs/pilots/cp7a)
"use strict";
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");

const METHOD = path.join(__dirname, "..", "method");
const FROZEN = "1E9671EC34F536F9E6070F0F791BE466A3ECB025BD2D5EDECE3081552047F93C";
const FROZEN_BACKUP = "0AD65573FE0BA0D6E6363247DDE84798F408F3A73BE7505E9FBFDCE287CCBE70";

let cfg = fs.readFileSync(path.join(METHOD, "global-opencode.original-backup.jsonc"), "utf8");
const backupHash = crypto.createHash("sha256").update(cfg, "utf8").digest("hex").toUpperCase();

// Edit 1: append the riltra local stdio server after context7.
const old1 = `      "context7": {
        "type": "remote",
        "url": "https://mcp.context7.com/mcp"
      }`;
const new1 = `      "context7": {
        "type": "remote",
        "url": "https://mcp.context7.com/mcp"
      },
      "riltra": {
        "type": "local",
        "command": [
          "E:\\\\dev\\\\Riltra\\\\target\\\\debug\\\\riltra.exe",
          "mcp"
        ],
        "environment": {
          "RILTRA_HOME": "E:\\\\dev\\\\Riltra\\\\.cp7a\\\\home-template"
        },
        "codemode": false
      }`;
// Edit 2: disable the context7 remote server for the run.
const old2 = `        "url": "https://mcp.context7.com/mcp"
      },`;
const new2 = `        "url": "https://mcp.context7.com/mcp",
        "disabled": true
      },`;

const e1 = cfg.includes(old1), e2raw = cfg.replace(old1, new1).includes(old2);
cfg = cfg.replace(old1, new1).replace(old2, new2);
const rebuiltHash = crypto.createHash("sha256").update(cfg, "utf8").digest("hex").toUpperCase();
const archivedHash = crypto.createHash("sha256").update(fs.readFileSync(path.join(METHOD, "global-opencode.cp7a.jsonc"))).digest("hex").toUpperCase();

console.log("backup sha256          =", backupHash, backupHash === FROZEN_BACKUP ? "(match)" : "(MISMATCH)");
console.log("edits applicable       =", e1 && e2raw);
console.log("reconstructed sha256   =", rebuiltHash);
console.log("archived config sha256 =", archivedHash);
console.log("frozen sha256          =", FROZEN);
const ok = e1 && e2raw && rebuiltHash === FROZEN && archivedHash === FROZEN;
console.log(ok ? "PASS: archived benchmark global config is byte-exact" : "FAIL: reconstruction does not match");
process.exit(ok ? 0 : 1);
