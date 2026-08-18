# Security Policy

Hydra modifies Git and filesystem state, so path validation, worktree
isolation, command execution, rollback, and recovery defects may have security
or data-loss impact even when they do not expose a network service.

## Supported versions

Hydra is an early preview. Security fixes are provided for the latest published
preview only; older previews and unreleased source snapshots are not supported
release lines.

| Version | Supported |
| --- | --- |
| Latest preview | Yes |
| Older previews | No |

## Report a vulnerability privately

Do not open a public issue for a suspected vulnerability. Use GitHub's private
vulnerability reporting flow from the repository Security tab when it is
available. If that flow is unavailable, contact
[@leonardoLoddo](https://github.com/leonardoLoddo) to arrange a private channel
before sharing sensitive details.

Include only the information needed to reproduce and assess the issue:

- affected Hydra version, operating system, filesystem, and Git version;
- the command and minimal sequence that triggers the problem;
- expected and observed Git, worktree, branch, file, or skill state;
- whether uncommitted work, credentials, or paths outside the managed project
  can be affected;
- a reproduction using a disposable repository when safely possible.

Remove secrets, access tokens, private source code, and unnecessary personal
paths. Do not exploit the issue against repositories or systems you do not own.

Reports are acknowledged and assessed on a best-effort basis. The maintainer
will coordinate validation, remediation, disclosure timing, and attribution
privately; the preview does not currently promise a fixed response SLA.
