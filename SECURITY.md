# Security Policy

## Supported versions

unplot is pre-1.0 and ships from the latest release. Security fixes land on
`main` and go out in the next tagged release. Please test against the newest
version before reporting.

## Reporting a vulnerability

**Please do not open a public issue for security problems.**

Report privately through GitHub's
[private vulnerability reporting](https://github.com/vitorwilson/unplot/security/advisories/new)
(the **Security → Report a vulnerability** button on the repository).

Please include:

- what the issue is and where (file, command, or feature),
- steps to reproduce, and
- the impact you think it has.

You'll get an acknowledgement, and once a fix is released the report will be
disclosed with credit unless you prefer to stay anonymous.

## Scope notes

unplot is an offline desktop app: it does no network I/O and stores curves as
local `.unplot` files you choose. The most relevant surface is therefore
malformed input — a hand-crafted `.unplot` document or pasted point data. Reports
about parsing, file handling, or the Tauri command boundary are especially
welcome.
