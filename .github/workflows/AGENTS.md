# Workflow Rules

## Standard

- `WF-001` Workflows MUST be split by lifecycle concern.
- `WF-002` Workflow filenames SHOULD use semantic names such as `ci.yml`, `deploy.yml`, and `release.yml`.
- `WF-003` Reusable workflow filenames MUST use the pattern `reusable-*.yml`.
- `WF-004` A `reusable-*.yml` workflow MUST use `workflow_call`.
- `WF-005` Jobs MUST represent execution boundaries.
- `WF-006` Independent jobs SHOULD run in parallel.
- `WF-007` `needs:` MUST only represent actual job dependencies.
- `WF-008` Matrices SHOULD only represent variations of the same job.
- `WF-009` Job dependencies, conditions, and orchestration SHOULD remain explicit in workflow YAML.
- `WF-010` Workflow and job identifiers MUST use simple semantic names.

## Preferences

- `WF-P001` Linux runners SHOULD be the default.
- `WF-P002` A job that can execute correctly on Linux SHOULD run on Linux.
- `WF-P003` macOS or Windows runners SHOULD only be used for platform-specific requirements or explicit compatibility testing.
- `WF-P004` `lint-pr.yml` MUST NOT be modified.
