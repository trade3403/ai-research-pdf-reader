# GitHub code versioning vs local research data

## Code (private GitHub repository)

- All source updates are committed to the connected private GitHub repository. GitHub contains code, configuration, tests, and documentation **only**; no production SQLite databases, private PDFs, Azure Key, or personal credentials.
- The `main` branch receives automatic Windows build/test runs on every push. These are CI artifacts, not stable releases.
- After CI passes, use **GitHub Actions → Publish Verified Windows Release → Run workflow** with `v0.5.1`. This manually approved workflow verifies that `main` HEAD has a successful Windows build, downloads its exact installer/checksum, verifies SHA-256, creates the tag, and publishes a GitHub Release. It never publishes a failing or untested commit. An independently pushed stable tag also triggers the existing tagged-build release workflow.
- Release artifacts may be downloaded again from GitHub Releases; the Git commit/tag preserves the source version.
- A new `Prepare Code Rollback Pull Request` workflow can be manually dispatched with an existing `vX.Y.Z` tag. It produces a reviewable PR and leaves `main` unchanged until reviewed. Never `git push --force` to restore a release.

## Research data (not Git)

- Research annotations, literature library, translations, and Word drafts live in the app's SQLite database.
- Click the existing database-backup command before upgrading. Keep **at least two copies** in separate locations and test a restore with non-critical data.
- The original PDF/DOCX documents are separate files. Back them up independently while preserving source folders.
- Azure Translator Key is stored in Windows Credential Manager and must be re-configured on a new computer; it is intentionally excluded from code and database backups.
- Restoring a Git tag does not restore or downgrade SQLite schema. Backwards migration is not guaranteed; test older application versions against **copies** of the database, never the sole live file.

## Tag release checklist

1. CI on `main` passes all V0.2–V0.5.1 tests and Windows NSIS build.
2. Compare commit hash, version and release notes.
3. Run `Publish Verified Windows Release` with `v0.5.1` on GitHub Actions; the workflow verifies the successful build and creates the tag/release.
4. Confirm GitHub Release contains the tested EXE and checksum.
5. Install on test Windows machine, verify native math, cross-references, tables and backups before using production data.
