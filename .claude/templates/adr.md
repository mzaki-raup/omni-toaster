# Template — ADR (append to `docs/08-decisions.md`)

One record per decision that a later reader would otherwise have to guess at: a
new dependency, an external endpoint, a storage format, a thread or process
structure, a platform restriction, a UI structure.

The ADR log **starts fresh in this repository**. Its first records are the ones
this project's realignment files — the Cargo workspace layout, the thread-based
concurrency model (no async runtime), eframe/egui as the UI framework, JAKIM
e-Solat as the default prayer source inside Malaysia, the supervisor and worker
split, and the in-app updater. Numbering continues from the last record in the
file; read the file rather than assuming a count. Status stays *Proposed* until
the first code lands against it.

---

```markdown
---

## ADR-0NN · <Short decision, in the file's voice>

**Context.** What forced the decision — the requirement, the constraint, the
thing that broke. One short paragraph.

**Options.** A · B · C — or a table where the trade-offs need columns:

| Option | For | Against |
|---|---|---|
| **<chosen>** | … | … |
| <rejected> | … | … |

**Decision.** The option, with the version or crate pinned where that matters
(`ureq` blocking rather than `reqwest`, which pulls in an async runtime this
project does not have; `tray-icon` with `muda` rather than `trayicon`, because
it is maintained alongside winit; JAKIM e-Solat as the default inside Malaysia
with `adhan` as the calculated fallback).

**Consequences.** What this costs: a dependency behind a Cargo feature, a
longer CI build, a Windows-only API that has to sit in `ot-platform`, a
`cargo deny` exception, an external endpoint whose format can change without
notice, work this now makes necessary elsewhere. Name the ADRs it interacts
with.
```

## Before writing one

- Check `cargo deny check` passes with the dependency (`deny.toml`), and prefer
  pure Rust and rustls.
- A dependency that drags in an async runtime needs a very good reason: there
  is no tokio in this project and the concurrency model is deliberate.
- A Windows-only crate means its calls live in `ot-platform` behind an
  interface, so clippy still passes on Linux. Say so in Consequences.
- An external endpoint that is not a documented public API — e-Solat is the
  case in point — needs its validation and fallback written into the ADR, not
  left to the implementation.
- Reference the ADR number from the spec and from the code comment that
  implements it.
