# IDE improvement plan

These documents turn the source review into separate, reviewable work items.
The status below records delivered work; each document retains its scope and verification notes.
Items 01–10 preserve the order of the original review, rather than prescribing the
implementation order. Step 00 defines the initial extraction and item 11 covers
verification. Each document includes its own acceptance criteria.

| Item | Work | Priority |
| --- | --- | --- |
| [00](00-minimal-controller-extraction.md) | Minimal document and execution lifecycle extraction | First enabling step |
| [01](01-recovery.md) | Recover unsaved documents without losing backups | Critical |
| [02](02-safe-saves-and-conflicts.md) | Atomic saves and conflict detection | High |
| [03](03-execution-workspaces.md) | Unique execution workspaces and durable pending uploads | High |
| [04](04-binary-transfers.md) | Binary assets in remote projects | High |
| [05](05-breakpoints-and-source-snapshots.md) | Breakpoint anchors and debugger source identity | High |
| [06](06-responsiveness.md) | Background storage and document analysis | High |
| [07](07-shared-language-analysis.md) | Shared, scope-aware language analysis | Medium |
| [08](08-classroom-server-boundaries.md) | Student identity and filesystem boundaries | High before shared classroom deployment |
| [09](09-repl-streaming.md) | Bounded REPL output and session lifecycle | High |
| [10](10-architecture-and-ownership.md) | Controllers, ownership, and explicit application states | Incremental foundation |
| [11](11-testing-and-verification.md) | Isolated tests and meaningful GUI verification | Cross-cutting |

## Current status

- 00 implemented: commit `d96e442`.
- 01 implemented: commit `dfadefa`.
- 02 implemented: commit `9289114`.
- 03 implemented; its navigation follow-up was committed as `021f0a6` and manually tested.
- 05 implemented in the working tree, ready for review; no commit created.
- 06 implemented in the working tree, ready for review; no commit created. See its storage inventory and recorded responsiveness measurements.
- 09 implemented in the working tree, ready for review; no commit created.
- 04, 07–08 and the remainder of 10 remain separate planned work; 11 supplies cross-cutting verification practices.

The earlier items still have manual desktop walkthroughs and native Windows
verification outstanding. Those checks are not blockers for implementing 03;
they remain review/release checks. See each item's verification section.

## Should item 10 come first?

Yes, through [step 00](00-minimal-controller-extraction.md). Extract the document
lifecycle operations and execution availability transitions needed by the first
fixes, while preserving behavior.
Do not make a complete architectural rewrite a prerequisite for addressing data loss.
If extraction delays a straightforward recovery fix, fix recovery first.

Recommended sequence:

1. Complete step 00, including the minimum verification setup from item 11.
2. Fix recovery (01), safe saves (02), and pending execution output (03).
3. Fix breakpoint tracking (05) and REPL streaming (09).
4. Complete background I/O (06) and binary transfers (04).
5. Establish server boundaries (08) before relying on shared student isolation.
6. Build shared language analysis (07), continuing item 10's extraction as needed.

Keep each change reviewable. For architectural changes, verify unchanged behavior;
for fixes, add regression tests that demonstrate the original failure. The order
can change to put server isolation first when classroom deployment is imminent.
