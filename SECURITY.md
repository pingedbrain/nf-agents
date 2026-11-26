# Security policy

## Supported versions

nf-agents is pre-alpha. Only the latest commit on `main` receives security fixes.

## Reporting a vulnerability

Please **do not** open a public issue for security problems.

Report vulnerabilities privately through GitHub: go to the repository's **Security** tab and click **Report a vulnerability**. Include:

- what is affected (crate, module, OpenTofu profile, workflow),
- steps to reproduce or a proof of concept,
- the impact you expect.

We aim to acknowledge reports within 7 days and to agree on a disclosure timeline with you.

## Scope notes

- Tools and agents are **read-only by default**. Anything that lets an agent modify data or infrastructure without explicit opt-in is a security bug.
- Anything that sends user data or telemetry outside the user's own cloud account without explicit opt-in is a security bug.
- Never include real patient or genomic data in a report.
