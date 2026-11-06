# User docs: rules for agents and contributors

This folder contains the downstream user documentation. No upstream website publishing or
community relationship is implied. Preserve relative page paths to avoid breaking local links.

## Who reads this

Artists using FDE2E Photocraft, maintainers answering them, and AI assistants looking things up for
either. Write for an artist, not a developer.

## Page rules

- One question or problem per page. Lead with the answer or the fix in the first two lines.
- Plain words, short sentences, numbered steps for anything the reader does.
- Start the body at `##`. The page title comes from frontmatter.
- Link to other pages instead of repeating them. Use relative links to the `.md` file:
  `[Install FDE2E Photocraft](../getting-started/install.md)`.
- No marketing copy, no "simply", no claims you have not checked in the code or a release.
- Call the product "FDE2E Photocraft". Link support to this fork, never upstream social channels.
- Name UI labels exactly as the app shows them, in **bold**. Menu paths read `File › Open…`.
- Troubleshooting pages: `## Symptoms`, `## Fix`, `## Why this happens` (optional),
  `## Still stuck?` (link the issue tracker for this fork).

## Frontmatter

    ---
    title: Export an image as PNG or JPEG
    description: One sentence for search results, link previews and AI summaries.
    order: 10                 # optional, sorts within the folder (lower first)
    verified: 0.7.0           # optional, app version you last checked this page against
    redirect_from: [old/path] # optional, old URLs (relative to /docs/photocraft)
    ---

Only `key: value` and `key: [a, b]` lines work. Quote values that start with a quote or contain ` #`.

Set `verified` to the latest release (the newest release tag) after checking the page against that tag with `git show <tag>:<path>`. Pages describe the latest release users download, not unreleased code on main.

## Folders

- `getting-started/` install and first run
- `guides/` how to do one thing
- `troubleshooting/` symptom → fix
- `faq/` short answers
- `reference/` lists and settings

A folder's `index.md` names the folder in the sidebar. Files or folders starting with `_` are
drafts and are not published.

## Check before you push

Check relative links and UI labels against this checkout. Run `bash scripts/brand-test.sh`.
The `verified` values inherited from upstream describe upstream PhotoCraft releases, not
releases published by FDE2E. Do not relabel historical fixes as FDE2E releases.

## Support PRs

Pages written to answer a user question come in as PRs titled `docs(support): <page title>`.
Review them like code: check every claim against the source.
