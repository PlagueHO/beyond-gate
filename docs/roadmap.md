# Implementation roadmap

The foundation provides help and version output only. All product work below is
pending. Each issue includes acceptance criteria, dependencies, tests, and
documentation requirements.

## Implementation issues

| Issue | Scope | Prerequisites |
|-------|-------|---------------|
| [#1](https://github.com/PlagueHO/beyond-gate/issues/1) | Permitted access and technical feasibility | None; stop gate |
| [#2](https://github.com/PlagueHO/beyond-gate/issues/2) | Typed HTTP client and bounded request policy | #1 |
| [#3](https://github.com/PlagueHO/beyond-gate/issues/3) | Simple human login and protected sessions | #1, #2 |
| [#4](https://github.com/PlagueHO/beyond-gate/issues/4) | Accessible book library discovery | #1, #2, #3 |
| [#5](https://github.com/PlagueHO/beyond-gate/issues/5) | Authorized character and campaign reads | #1, #2, #3 |
| [#6](https://github.com/PlagueHO/beyond-gate/issues/6) | Book outlines and bounded section reads | #4 |
| [#7](https://github.com/PlagueHO/beyond-gate/issues/7) | Resource-first CLI, output formats, completions | #5, #6 |
| [#8](https://github.com/PlagueHO/beyond-gate/issues/8) | Local stdio MCP and read parity | #5, #6; coordinate with #7 |
| [#9](https://github.com/PlagueHO/beyond-gate/issues/9) | Local MCP start and install | #7, #8 |
| [#10](https://github.com/PlagueHO/beyond-gate/issues/10) | Native packages and first-run usability | #9 and its prerequisites |

Implement the HTTP boundary before full session exchange. Simple login remains
an early required milestone, not an optional fallback. Library discovery and
character/campaign reads can proceed in parallel after session handling.
The adapters can follow each completed core read without waiting for all reads.

The feasibility work is a stop gate, not permission to bypass a restriction.
If a path is not permitted or viable, record that result and revise scope before
dependent live integration. Synthetic offline tests can still be developed.

## Release boundary

v1 is local-only and read-only. It needs simple authentication, consistent CLI/MCP
results, source URLs, bounded book reads, and clear errors before a usable release.
There is no requirement for a cloud service, Python, Node.js, or crate publication.
