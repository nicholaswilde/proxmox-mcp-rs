# Implementation Plan - Gemini Release Summary with Gitmoji

## Phase 1: Data Collection & Preparation
- [ ] Task: Create a script/step to gather changelog data since the last tag.
    - [ ] Sub-task: Write a shell script `scripts/get_changelog_data.sh` that uses `git log` and `git diff` to extract commits, PR references, and statistics.
    - [ ] Sub-task: Implement a test script to verify the output format of the changelog data.
- [ ] Task: Conductor - User Manual Verification 'Data Collection' (Protocol in workflow.md)

## Phase 2: Gemini Integration
- [ ] Task: Implement the Gemini API call to generate the summary.
    - [ ] Sub-task: Create a Python or Node.js script `scripts/generate_release_summary.py` that takes changelog data and sends it to `gemini-2.0-flash` with a system prompt for gitmoji categorization.
    - [ ] Sub-task: Write mock-based tests for the summary generator to ensure it handles various commit patterns and API responses.
- [ ] Task: Conductor - User Manual Verification 'Gemini Integration' (Protocol in workflow.md)

## Phase 3: Workflow Automation
- [ ] Task: Update `.github/workflows/release.yml` to integrate the generation logic.
    - [ ] Sub-task: Add a new job/step to run the scripts from Phase 1 & 2.
    - [ ] Sub-task: Update the `gh release create` command to include the generated summary as the `--notes` argument.
- [ ] Task: Conductor - User Manual Verification 'Workflow Automation' (Protocol in workflow.md)
