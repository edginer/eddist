---
name: commit-and-push
description: >
  Formats and lints the codebase, then creates a git branch, commits, and pushes.
  Use this skill whenever the user wants to commit changes, push code, create a branch,
  or says things like "commit this", "push my changes", "create a branch and commit",
  "let's push", "save my work to git", or anything implying a git commit/push workflow.
  Always invoke this skill before doing a raw `git commit` or `git push`.
allowed-tools:
  - AskUserQuestion
  - Bash(git status)
  - Bash(git diff*)
  - Bash(git log*)
  - Bash(git branch*)
  - Bash(git checkout*)
  - Bash(git add*)
  - Bash(git commit*)
  - Bash(cargo fmt)
  - Bash(cargo sqlx prepare --workspace)
  - Bash(pnpm -F eddist-admin-client check)
  - Bash(pnpm -F eddist-client-v2 check)
  - Bash(pnpm -F eddist-admin-client typecheck)
  - Bash(pnpm -F eddist-client-v2 typecheck)
---

# Commit and Push Skill

This skill handles the full pre-commit formatting and git workflow for the eddist monorepo.

## Step 1: Gather info with AskUserQuestion

Before doing anything, ask the user for:
- **Branch name** — the new branch to create (e.g. `feat/my-feature`, `fix/some-bug`)
- **Commit message** — a detailed message following conventional commits style. The subject line should be concise (e.g. `feat: add X`, `fix: correct Y`), followed by a body that explains *what* changed and *why* (bullet points are fine). Example:
  ```
  fix: correct auth token validation in bbs_cgi

  - Fix missing check for suspended tokens in bbscgi_auth_service
  - Update error mapping in error.rs to surface auth failures correctly
  - Align auth_with_code_service with updated pubsub_repository interface
  ```

If the user already provided one or both of these in their message, skip asking for them.

## Step 2: Run Rust formatter

```
cargo fmt
```

This formats all Rust code in the workspace. Must succeed before continuing.

## Step 3: Run SQLx prepare

```
cargo sqlx prepare --workspace
```

This regenerates the `.sqlx/` offline query cache. Must succeed before continuing.
If it fails due to no DATABASE_URL, note that to the user and skip this step.

## Step 4: Run frontend format & lint (Biome)

Run biome check with auto-fix for both frontend packages:

```
pnpm -F eddist-admin-client check
pnpm -F eddist-client-v2 check
```

These run `biome check --write ./app` which formats and lints in one pass.
Run both; if either has lint errors that cannot be auto-fixed, report them to the user and stop.

## Step 4b: Run frontend TypeScript typechecks

Run TypeScript typechecks for both frontend packages:

```
pnpm -F eddist-admin-client typecheck
pnpm -F eddist-client-v2 typecheck
```

These run `react-router typegen && tsc` which regenerates route types then checks the full TypeScript project.
Run both; if either reports type errors, report them to the user and stop — do not commit with type errors.

## Step 5: Git workflow

```
git checkout -b <branch-name>
git add -A
git commit -m "<commit-message>"
```

Use the branch name and commit message gathered in Step 1.

**Important:** Always use `git add -A` to stage ALL changes (including any files that may have been missed) in a single commit. Never create a follow-up commit to add forgotten files — if files were missed, amend the commit instead.

## Step 6: Push

```
git push -u origin <branch-name>
```

## Notes

- Always run the formatters **before** staging files so that formatted changes are included in the commit.
- If `cargo sqlx prepare` fails because DATABASE_URL is not set, skip it and inform the user — they may need to run it separately against a live database.
- If biome reports unfixable lint errors, stop and show the errors to the user rather than committing broken code.
