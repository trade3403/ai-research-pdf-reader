# Electron V0.6.0 migration bootstrap
Separate preview app: existing Tauri and React sources are untouched. This is not a feature-complete replacement. A strict ChatGPT origin allowlist intentionally blocks auth redirects; use external browser until a reviewed auth flow is implemented. No local data is exposed to the remote view. No PDF content is loaded yet.
Run: cd electron-preview; npm install; npm test; npm start. Build: npm run dist.
Initial dependency ranges must be resolved and lockfile committed before release; CI currently uses npm install for bootstrap. Next: React/Vite integration, platform adapter, authenticated navigation tests, SQLite/credential migration, RPCore worker integration, installation smoke tests.
