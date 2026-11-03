# SRD source data

The raw data the SRD pack is built from, in the 5e-database JSON format. Kept in this repo so the app builds without network access or other checkouts.

| What | Where it came from |
|---|---|
| `2024/en/*.json` (all files except `5e-SRD-Rules.json`) | [5e-bits/5e-srd-api](https://github.com/5e-bits/5e-srd-api), `packages/5e-database/src/2024/en`, version **5e-database-v7.9.1**, commit `67d8c6aad9a273862fc5ff5f633fcdc06130b94c`. Not modified. |
| `2024/en/5e-SRD-Rules.json` | Built by us from the SRD 5.2.1 PDF in the same format as 5e-database's 2014 `5e-SRD-Rules.json` (not in 5e-srd-api yet). |
| `LICENSE.md` | The MIT license of 5e-srd-api's `packages/5e-database`. |

SRD 5.2.1 PDF: https://media.dndbeyond.com/compendium-images/srd/5.2/SRD_CC_v5.2.1.pdf
SHA-256 `8974902d109d6e63672d7c490bde9ccf052410503d9cfa768237154fbc5e3d87`.

Do not edit the copied files by hand; replace them with a newer 5e-database release instead (Settings → Update rules does the same at runtime, in the app's data folder).
