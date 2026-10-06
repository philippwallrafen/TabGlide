# Composite Action Rules

## Standard

- `ACT-001` Each composite action MUST be stored in its own directory under `.github/actions/`.
- `ACT-002` Each composite action entrypoint MUST be named `action.yml`.
- `ACT-003` Composite actions SHOULD represent reusable sequences of workflow steps.
- `ACT-004` Each composite action SHOULD have one primary responsibility.
- `ACT-005` Composite action directories MUST use simple semantic names.
- `ACT-006` Composite actions SHOULD NOT be created solely to wrap a trivial single command.

## Preferences

- `ACT-P001` Composite actions SHOULD remain compatible with Linux runners where practical.
- `ACT-P002` Platform-specific behavior SHOULD be avoided unless required by the action.
