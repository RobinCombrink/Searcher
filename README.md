# Searcher

CLI tool for searching code across multiple git repositories.

## What it does

Clones repositories from a GitHub organisation (filtered by name prefix), then performs regex searches across all of them with coloured terminal output. Useful for finding patterns, usages, or code across a large set of related repositories without manually cloning and grepping each one.

## Tech Stack

- **Rust** — core language
- **grep** (ripgrep's library) — fast regex-based file searching
- **git2/libgit2** (via `git_cloner`) — repository cloning and branch checkout
- **octocrab** — GitHub API client for listing organisation repos
- **clap** — CLI argument parsing
- **tokio** — async runtime for concurrent operations

## Running locally

Requires the GitHub CLI (`gh`) to be installed and authenticated.

```bash
# Search for a pattern across repos in an org
cargo run -- search -o my-org -p repo-prefix -s "pattern" -b main,develop

# Case-sensitive search
cargo run -- search -o my-org -s "ExactMatch" -i false

# Clean up cloned repos
cargo run -- clear
```

## Design Decisions

- **ripgrep's grep library over shelling out**: Uses the same engine as ripgrep directly as a library, getting fast search with proper binary file detection and coloured output without process spawning.
- **Clone-then-search over GitHub search API**: GitHub's code search has rate limits and indexing delays. Local cloning gives complete, immediate results across all branches.
- **Shared `git_cloner` crate**: Repository cloning logic is extracted into a reusable crate rather than inlined, keeping the search logic focused on search.
