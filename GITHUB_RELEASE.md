# AI Research PDF Reader — Windows build

Open **Actions > Build and Publish Windows Installer** and select the most recent successful run.
Download the `AI-Research-PDF-Reader-Windows-Installer` artifact ZIP from the bottom of the run page.
The artifact contains the NSIS Setup `.exe` and SHA256SUMS.txt.

This V0.1 installer is not code-signed; Windows SmartScreen may display a warning. Review the build source and installer before proceeding.

Developer source: `npm install`, `npm run tauri:dev`; Windows NSIS: `npm run tauri:build`.

The PDF reader currently opens local PDFs only; there is no AI API integration or Zotero sync in V0.1.
