# Implementation Plan - Gemini Release Summary with Gitmoji

## Phase 1: Data Collection & Preparation
- [ ] Task: Create a script/step to gather changelog data since the last tag.
    - [ ] Sub-task: Write a shell script `scripts/get_changelog_data.sh` that uses `git log` and `git diff` to extract commits, PR references, and statistics.
    - [ ] Sub-task: Implement a test script to verify the output format of the changelog data.
- [ ] Task: Conductor - User Manual Verification 'Data Collection' (Protocol in workflow.md)

## Phase 2: Gemini Integration
- [ ] Task: Configure the `run-gemini-cli` action to generate the summary.
    - [ ] Sub-task: Draft a system prompt for Gemini that instructs it to use gitmojis and group changes by type.
    - [ ] Sub-task: Create a test workflow file `.github/workflows/test-gemini-summary.yml` to verify the action configuration and prompt effectiveness using the `run-gemini-cli` action.
- [ ] Task: Conductor - User Manual Verification 'Gemini Integration' (Protocol in workflow.md)

## Phase 3: Workflow Automation
- [ ] Task: Update `.github/workflows/release.yml` to integrate the generation logic.
    - [ ] Sub-task: Add the `google-github-actions/run-gemini-cli` step to the workflow, passing the changelog data as input.
    - [ ] Sub-task: Update the `gh release create` command to use the output from the Gemini action as the release notes.
- [ ] Task: Conductor - User Manual Verification 'Workflow Automation' (Protocol in workflow.md)
