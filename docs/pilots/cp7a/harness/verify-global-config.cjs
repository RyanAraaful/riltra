// Reconstruct the benchmark-time OpenCode global config from the archived original
// backup plus the two edits recorded in the session history, then verify it is
// byte-identical to the archived benchmark config. The archived copy is a sanitized
// derivative (local machine paths replaced by placeholders; see SANITIZATION.md), so
// it must match DERIVATIVE below, while FROZEN keeps the pre-redaction SHA-256
// 1E9671EC... recorded in method/environment.json before measurement.
// Run: node harness/verify-global-config.cjs   (from docs/pilots/cp7a)
"use strict";
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");

const METHOD = path.join(__dirname, "..", "method");
const FROZEN = "1E9671EC34F536F9E6070F0F791BE466A3ECB025BD2D5EDECE3081552047F93C";
const FROZEN_BACKUP = "0AD65573FE0BA0D6E6363247DDE84798F408F3A73BE7505E9FBFDCE287CCBE70";
const DERIVATIVE = "D4D892E6CB7F0731543C0B5826CDA60E0990A662D7F00CC382A9F3DF4AA17C96";

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
          "<RILTRA_REPO_ROOT>\\\\target\\\\debug\\\\riltra.exe",
          "mcp"
        ],
        "environment": {
          "RILTRA_HOME": "<CP7A_ROOT>\\\\home-template"
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
console.log("frozen sha256 (pre-redaction)  =", FROZEN);
console.log("derivative sha256      =", DERIVATIVE);
const ok = e1 && e2raw && rebuiltHash === DERIVATIVE && archivedHash === DERIVATIVE;
console.log(ok ? "PASS: archived benchmark global config is byte-exact (sanitized derivative)" : "FAIL: reconstruction does not match");
process.exit(ok ? 0 : 1);
