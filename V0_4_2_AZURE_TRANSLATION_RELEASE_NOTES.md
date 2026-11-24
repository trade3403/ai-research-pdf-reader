# AI Research PDF Reader V0.4.2 — Azure Translator F0 Integration

## What changed

- PDF.js reader selection toolbar: **翻译** for selected sentences/paragraphs.
- ChatGPT research records: **翻译 / 中英对照** shows original and translated Markdown/LaTeX side by side.
- Optional Azure Translator F0 (Azure single-service global resource or regional resource).
- Translator Key stored in **Windows Credential Manager** via `keyring` native backend. Never in SQLite, app config, logs or bundled source.
- Azure requests made **only by the trusted local Rust Tauri backend** to a fixed Microsoft HTTPS URL. The remote ChatGPT WebView is not given Tauri permissions.
- Local SQLite `translation_preferences`, `translation_cache`, and `translation_usage` tables; additive migration of previous database.
- UTC calendar-month client-side budget (default **1,500,000** characters, adjustable 1,000 to 2,000,000), quota reservation before requests, SHA-256 input+language+glossary caching.
- LaTeX (`$...$`, `$$...$$`, `\\(...\\)`, `\\[...\\]`), fenced code and inline code are excluded from translation payload and restored locally. Formula and Markdown preview use existing KaTeX renderer.
- Custom local glossary in `English = 中文` format; deterministic term replacement and no API translation of terms. Glossary terms may interrupt longer sentence context.
- Reusable `translateResearchContent()` accepts plaintext/Markdown input from PDF, notes, or future Word adapter.

## Configure

1. In [Microsoft Azure Portal](https://portal.azure.com/), create an **Azure AI Translator** resource using **F0** tier, subject to account availability.
2. Copy your **Key** and **Resource Region** from **Keys and Endpoint**. For a global single-service Translator resource, use `global`; for regional/multiservice resources, use the region (e.g., `eastasia`).
3. In the desktop application click **翻译设置**, enter Region + Key, set the monthly local budget, then save.
4. Open a PDF, select text and press **翻译**, or open a saved ChatGPT research record and click **翻译 / 中英对照**.
5. Repeating an identical input and configuration returns locally cached translation and does not make an Azure request.

## Safety, limitations and acceptance

- No Azure Key is included in any release. Users provide their own; never share keys in a public chat/repository.
- The app sends selected **natural-language text** to Azure. Do not translate confidential/unpublished material without checking your organization's policies and Azure processing terms.
- Azure F0 offers **2 million characters per month** for standard translation, subject to subscription and pricing terms; verify current quotas in Azure Portal.
- Local quota counters are conservative **pre-request reservations**, not authoritative Azure billing or account-level balances. Failed requests may still count locally; usage from other Azure clients is not included.
- Not an OCR or full document translation feature; PDF math extraction is best effort and may not reproduce equation structure.
- Markdown/LaTeX/regex segmentation is best effort. Formula/code must be reviewed before publication.
- No real Azure API call or embedded ChatGPT login is tested in CI, because no user key or authenticated Windows environment is available.
- Verification: `npm run test:v02`, `test:v03`, `test:v04`, `test:math`, `test:translation`, `npm run tauri:build`, NSIS binary PE signature and SHA-256.

## Existing data

- Uses the original `research-reader-v02.db` database and app identifier. Previous annotations, reading state, library and research clips remain intact.
- Back up your app-data database before upgrading. Changing translation preferences never edits source PDF or research clip bodies.
