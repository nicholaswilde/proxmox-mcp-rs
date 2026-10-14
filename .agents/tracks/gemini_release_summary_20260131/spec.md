# Specification - Gemini Release Summary with Gitmoji

## Overview
Update the GitHub release workflow to automatically generate a human-readable release summary using Google Gemini. The summary will use `gitmoji` to categorize changes, making releases more informative and visually consistent.

## Functional Requirements
- **Gemini Integration:** Use the `gemini-2.0-flash` model via the `google-github-actions/run-gemini-cli` GitHub Action.
- **Data Source:** Provide Gemini with git commit messages, PR titles, and diff statistics since the last release tag.
- **Categorization:** Instruct Gemini to group changes using appropriate `gitmoji` (e.g., :sparkles: for `feat`, :bug: for `fix`, :recycle: for `refactor`, :memo: for `docs`).
- **Automatic Drafting:** The generated summary must be used as the body of the GitHub release created by the workflow.
- **Format:** Bulleted list grouped by change type.

## Technical Requirements
- **API Key:** Requires a `GEMINI_API_KEY` stored in GitHub Secrets.
- **Workflow Update:** Modify `.github/workflows/release.yml`.
- **Logic:** Add a job or step to fetch changelog data, call Gemini, and update the release.

## Acceptance Criteria
- Triggering a release (via tag push or workflow dispatch) generates a release body.
- The release body contains gitmojis correctly categorized.
- The summary accurately reflects the commits since the previous tag.
