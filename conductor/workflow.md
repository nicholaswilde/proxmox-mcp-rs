# Project Workflow

## Guiding Principles

1. **The Plan is the Source of Truth:** All work must be tracked in `plan.md`
2. **The Tech Stack is Deliberate:** Changes to the tech stack must be documented in `tech-stack.md` *before* implementation
3. **Test-Driven Development:** Write unit tests before implementing functionality
4. **High Code Coverage:** Aim for >80% code coverage for all modules
5. **User Experience First:** Every decision should prioritize user experience
6. **Non-Interactive & CI-Aware:** Prefer non-interactive commands. Use `CI=true` for watch-mode tools (tests, linters) to ensure single execution.

## Task Workflow

All tasks follow a strict lifecycle:

### Standard Task Workflow

1. **Select Task:** Choose the next available task from `plan.md` in sequential order

2. **Mark In Progress:** Before beginning work, edit `plan.md` and change the task from `[ ]` to `[~]`

3. **Write Failing Tests (Red Phase):**
   - Create a new test file for the feature or bug fix.
   - Write one or more unit tests that clearly define the expected behavior and acceptance criteria for the task.
   - **CRITICAL:** Run the tests and confirm that they fail as expected. This is the "Red" phase of TDD. Do not proceed until you have failing tests.

4. **Implement to Pass Tests (Green Phase):**
   - Write the minimum amount of application code necessary to make the failing tests pass.
   - Run the test suite again and confirm that all tests now pass. This is the "Green" phase.

5. **Refactor (Optional but Recommended):**
   - With the safety of passing tests, refactor the implementation code and the test code to improve clarity, remove duplication, and enhance performance without changing the external behavior.
   - Rerun tests to ensure they still pass after refactoring.

6. **Verify Coverage:** Run coverage reports using the project's chosen tools.
   Target: >80% coverage for new code. The specific tools and commands will vary by language and framework.

7. **Document Deviations:** If implementation differs from tech stack:
   - **STOP** implementation
   - Update `tech-stack.md` with new design
   - Add dated note explaining the change
   - Resume implementation

8. **Mark Task Complete in Plan:**
    - Read `plan.md`, find the line for the completed task, and update its status from `[~]` to `[x]`.
    - **Note:** Changes are committed at the end of each phase, not after each task.

### Phase Completion Verification and Checkpointing Protocol

**Trigger:** This protocol is executed immediately after a task is completed that also concludes a phase in `plan.md`.

1.  **Announce Protocol Start:** Inform the user that the phase is complete and the verification and checkpointing protocol has begun.

2.  **Commit Phase Changes:**
    - Stage all code and plan changes related to the phase.
    - Propose a clear, concise commit message summarizing the phase (e.g., `feat(node): Implement node listing and status tools`).
    - Perform the commit.

3.  **Attach Phase Summary with Git Notes:**
    - **Step 3.1: Get Commit Hash:** Obtain the hash of the *just-completed phase commit* (`git log -1 --format="%H"`).
    - **Step 3.2: Draft Note Content:** Create a detailed summary for the completed phase. This should include the tasks completed, a summary of changes, and a list of all created/modified files.
    - **Step 3.3: Attach Note:** Use the `git notes` command to attach the summary to the commit.
     ```bash
     git notes add -m "<note content>" <commit_hash>
     ```

4.  **Verify and Create Tests:**
    - **Step 4.1: Determine Phase Scope:** List all files changed in this phase since the last checkpoint.
    - **Step 4.2: Verify Tests:** Ensure every modified code file has corresponding tests validating the functionality described in the phase's tasks.

5.  **Execute Automated Tests with Proactive Debugging:**
    - Announce and execute the test suite.
    - If tests fail, debug and fix (maximum two attempts).

6.  **Propose a Detailed, Actionable Manual Verification Plan:**
    - Analyze `product.md`, `product-guidelines.md`, and `plan.md` to generate a step-by-step verification plan for the user.

7.  **Await Explicit User Feedback:**
    - Pause for user confirmation before proceeding.

8.  **Create Checkpoint Commit (if additional changes made during verification):**
    - Stage any fixes or verification artifacts.
    - Perform a checkpoint commit (e.g., `conductor(checkpoint): Checkpoint end of Phase X`).

9.  **Get and Record Phase Checkpoint SHA:**
    - Record the SHA of the final phase/checkpoint commit in `plan.md` next to the phase heading.

### Quality Gates

Before marking any task complete, verify:

- [ ] All tests pass
- [ ] Code coverage meets requirements (>80%)
- [ ] Code follows project's code style guidelines (as defined in `code_styleguides/`)
- [ ] All public functions/methods are documented (e.g., docstrings, JSDoc, GoDoc)
- [ ] Type safety is enforced (e.g., type hints, TypeScript types, Go types)
- [ ] No linting or static analysis errors (using the project's configured tools)
- [ ] Works correctly on mobile (if applicable)
- [ ] Documentation updated if needed
- [ ] No security vulnerabilities introduced

## Development Commands

### Setup
```bash
# Ensure Rust and task are installed
# https://rustup.rs/
# https://taskfile.dev/
```

### Daily Development
```bash
task check      # Check for compilation errors
task test       # Run unit tests
task fmt        # Format code using rustfmt
task lint       # Run lints using clippy
task build:debug # Build the project in debug mode
```

### Before Committing
```bash
task test:ci    # Run all formatting, linting, and tests at once
```

## Special Procedures

### Live Testing Workflow
When asked to test new functions since the last tag against a *live, connected* Proxmox server:

1.  **Identify Changes:** Run `git diff <last_tag> HEAD -- src/mcp.rs` to see which new tools were added.
2.  **Verify Config:** Ensure a `config.toml` exists or environment variables are set to connect to a real Proxmox instance.
3.  **Build:** Run `cargo build --release`.
4.  **Script Interaction:** Create a Python script (`test_mcp_live.py`) to spawn the binary and send JSON-RPC requests to the stdio transport.
    *   *Template Script:*
        ```python
        import subprocess, json, sys
        def rpc(method, params=None, id=1): return {"jsonrpc": "2.0", "method": method, "params": params, "id": id}
        cmd = ["./target/release/proxmox-mcp-rs"]
        p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=sys.stderr, text=True, bufsize=0)
        
        # 1. Init
        p.stdin.write(json.dumps(rpc("initialize", {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "1.0"}})) + "\n")
        print(p.stdout.readline()) # Read Init Response

        # 2. Call Tool (Example)
        # p.stdin.write(json.dumps(rpc("tools/call", {"name": "new_tool_name", "arguments": {...}})) + "\n")
        # print(p.stdout.readline())
        
        p.terminate()
        ```
5.  **Execute:** Run the script and verify the JSON-RPC responses indicate success.
6.  **Cleanup:** Remove the test script.

### Release Summary Guidelines
*   When asked for a GitHub release summary from the previous git tag to the current one, only summarize the MCP server functionality. Chore and documentation updates should be excluded.

## Testing Requirements

### Unit Testing
- Every module must have corresponding tests.
- Use appropriate test setup/teardown mechanisms (e.g., fixtures, beforeEach/afterEach).
- Mock external dependencies.
- Test both success and failure cases.

### Integration Testing
- Test complete user flows
- Verify database transactions
- Test authentication and authorization
- Check form submissions

### Mobile Testing
- Test on actual iPhone when possible
- Use Safari developer tools
- Test touch interactions
- Verify responsive layouts
- Check performance on 3G/4G

## Code Review Process

### Self-Review Checklist
Before requesting review:

1. **Functionality**
   - Feature works as specified
   - Edge cases handled
   - Error messages are user-friendly

2. **Code Quality**
   - Follows style guide
   - DRY principle applied
   - Clear variable/function names
   - Appropriate comments

3. **Testing**
   - Unit tests comprehensive
   - Integration tests pass
   - Coverage adequate (>80%)

4. **Security**
   - No hardcoded secrets
   - Input validation present
   - SQL injection prevented
   - XSS protection in place

5. **Performance**
   - Database queries optimized
   - Images optimized
   - Caching implemented where needed

6. **Mobile Experience**
   - Touch targets adequate (44x44px)
   - Text readable without zooming
   - Performance acceptable on mobile
   - Interactions feel native

## Commit Guidelines

### Message Format
```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Types
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting, missing semicolons, etc.
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `test`: Adding missing tests
- `chore`: Maintenance tasks

### Examples
```bash
git commit -m "feat(auth): Add remember me functionality"
git commit -m "fix(posts): Correct excerpt generation for short posts"
git commit -m "test(comments): Add tests for emoji reaction limits"
git commit -m "style(mobile): Improve button touch targets"
```

## Definition of Done

A task is complete when:

1. All code implemented to specification
2. Unit tests written and passing
3. Code coverage meets project requirements
4. Documentation complete (if applicable)
5. Code passes all configured linting and static analysis checks
6. Works beautifully on mobile (if applicable)
7. Implementation notes added to `plan.md`
8. Changes committed with proper message
9. Git note with task summary attached to the commit

## Emergency Procedures

### Critical Bug in Production
1. Create hotfix branch from main
2. Write failing test for bug
3. Implement minimal fix
4. Test thoroughly including mobile
5. Deploy immediately
6. Document in plan.md

### Data Loss
1. Stop all write operations
2. Restore from latest backup
3. Verify data integrity
4. Document incident
5. Update backup procedures

### Security Breach
1. Rotate all secrets immediately
2. Review access logs
3. Patch vulnerability
4. Notify affected users (if any)
5. Document and update security procedures

## Deployment Workflow

### Pre-Deployment Checklist
- [ ] All tests passing
- [ ] Coverage >80%
- [ ] No linting errors
- [ ] Mobile testing complete
- [ ] Environment variables configured
- [ ] Database migrations ready
- [ ] Backup created

### Deployment Steps
1. Merge feature branch to main
2. Tag release with version
3. Push to deployment service
4. Run database migrations
5. Verify deployment
6. Test critical paths
7. Monitor for errors

### Post-Deployment
1. Monitor analytics
2. Check error logs
3. Gather user feedback
4. Plan next iteration

## Continuous Improvement

- Review workflow weekly
- Update based on pain points
- Document lessons learned
- Optimize for user happiness
- Keep things simple and maintainable
