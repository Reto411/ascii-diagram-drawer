# arc42 — ascii-diagram-drawer

| | |
|---|---|
| **Document status** | Draft — being populated iteratively |
| **Version** | 0.1 |
| **Date** | 2025-10-16 |
| **Author(s)** | |
| **Target audience** | Developers, reviewers |

> **How to use this document:** Each section contains italic guidance text (what belongs here) and `[TBD]` placeholders. Fill sections incrementally; keep the progress table below updated. Remove guidance notes as sections are finalized.

## Progress tracker

| § | Section | Status |
|---|---------|--------|
| 1 | Introduction and Context | 🟨 (1.1 draft) |
| 2 | Architecture Vision | ⬜ |
| 3 | Business Scope | ⬜ |
| 4 | Stakeholders and Users | ⬜ |
| 5 | Constraints | ⬜ |
| 6 | Quality Model | ⬜ |
| 7 | Solution Strategy | ⬜ |
| 8 | Building Block View | ⬜ |
| 9 | Runtime View | ⬜ |
| 10 | Deployment View | ⬜ |
| 11 | Cross-cutting Concerns | ⬜ |
| 12 | Concepts | ⬜ |
| 13 | Building Blocks (detail) | ⬜ |
| 14 | Identification of Important Requirements | ⬜ |
| 15 | Risks and Technical Risks | ⬜ |
| 16 | Glossary | ⬜ |

---

## 1. Introduction and Context

> *What is this project, what problem does it solve, and in which world does it live?*

### 1.1 Objectives

> *Project goals in 3–7 bullet points. What must the finished product be able to do? Why does it exist?*

**Draft — pending confirmation:**

1. **Draw ASCII diagrams visually.** Users compose diagrams by placing, moving, and resizing predefined shapes (boxes, circles, …) on a canvas — instead of hand-typing box-drawing characters.
2. **Plain-text output.** The result is pure monospace text that can be copied and pasted anywhere: terminals, READMEs, chat, commit messages, docs.
3. **Reusable shape templates.** Users can define their own shapes and group them into collections for reuse across diagrams.
4. **Local desktop app.** Runs as a local desktop application (Tauri) — no server, works offline.
5. **Vector/raster export.** Unlike the previous project, diagrams can be rendered as SVG (vector) and PNG (raster) in addition to plain text.
6. **Low learning curve.** Minimal UI: a template sidebar to pick shapes from and an editor canvas to compose the diagram.

### 1.2 Quality Goals (top-level)

> *The 3–5 most important quality attributes of this system (e.g. "fast rendering", "zero-dependency output", "works offline"). Detailed scenarios go in §6.*

| # | Quality goal | Priority (H/M/L) |
|---|--------------|------------------|
| Q1 | [TBD] | |
| Q2 | [TBD] | |
| Q3 | [TBD] | |

### 1.3 Constraints

> *Hard limits that shape the design: tech stack, target platforms, performance budgets, licensing, offline requirements.* See §5 for details.

- [TBD]

### 1.4 Stakeholders

> *Who cares about this project and why? (users, developers, operators).* Details in §4.

| Stakeholder | Interest / concern |
|-------------|--------------------|
| [TBD] | |

---

## 2. Architecture Vision

> *The "big picture" before details: which quality goals drive the design, what technical risks exist, and which cross-cutting concepts unify the solution.*

### 2.1 Constraints (design-level)

> *Constraints from §1.3 that directly influence architectural decisions — restate only the relevant ones with their impact on the design.*

- [TBD]

### 2.2 Key Quality Goals

> *Restate the quality goals from §1.2 that drive architecture, in measurable form where possible (e.g. "first render < 100 ms for diagrams up to N nodes").*

| # | Key quality goal | Measurable target |
|---|------------------|-------------------|
| [TBD] | | |

### 2.3 Prioritization of Quality Goals

> *Order the key quality goals by importance and note conflicts between them (e.g. "simplicity vs. extensibility").*

1. [TBD]
2. [TBD]
3. [TBD]

Conflicts & trade-offs:

- [TBD]

### 2.4 Technical Risks

> *Top technical risks at vision level (e.g. "layout algorithm for ASCII boxes is unproven"). Detailed risk register in §15.*

| # | Risk | Impact | Likelihood | Mitigation idea |
|---|------|--------|------------|-----------------|
| R1 | [TBD] | | | |

### 2.5 Cross-cutting Concepts

> *Concepts that span multiple building blocks (e.g. "everything is a plain-text AST", "single source of truth: the diagram model").*

- [TBD]

---

## 3. Business Scope

> *What the product is from an outside perspective — no technical language.*

### 3.1 Context

> *Where does this fit in the user's workflow? What do users do before/after using it?*

- [TBD]

### 3.2 Products and Features

> *List of features / user-visible capabilities (not implementation).*

| # | Feature | Description |
|---|---------|-------------|
| F1 | [TBD] | |

### 3.3 User Characteristics

> *Who uses it? Skills, expectations, environment (e.g. "developers comfortable with ASCII art and monospace fonts").*

- [TBD]

---

## 4. Stakeholders and Users

> *Concrete list of stakeholders and user groups, plus requirements from their perspective.*

### 4.1 List of Stakeholders

| Stakeholder | Role | Interest / concern |
|-------------|------|--------------------|
| [TBD] | | |

### 4.2 List of Users / User Groups

| User group | Description | Primary need |
|------------|-------------|--------------|
| [TBD] | | |

### 4.3 Requirements from the Perspective of Stakeholders/Users

> *Free-form list of expectations per stakeholder/user group (will be formalized in §14).*

- [TBD]

---

## 5. Constraints

> *All hard constraints, grouped by category.*

### 5.1 Legal Constraints

> *Licensing (project license + dependency policy), copyright, data protection.*

- [TBD]

### 5.2 Boundary Conditions

> *Input size limits, supported character sets / Unicode, terminal/screen sizes, browser support.*

- [TBD]

### 5.3 Technical Framework Conditions

> *Mandated or assumed technology: e.g. SvelteKit + TypeScript + Vite (inferred from repo — confirm), Node.js version, build tooling, package manager.*

- [TBD]

---

## 6. Quality Model

> *The complete set of quality attributes for this system and how they are measured/verified.*

### 6.1 Quality Tree

> *Decompose the key quality goals (§2.2) into a tree of measurable sub-goals. A simple indented list is fine.*

```
Quality goal
├── sub-goal (measurable)
│   └── ...
└── sub-goal (measurable)
```

- [TBD]

### 6.2 Quality Scenarios

> *One scenario per quality attribute: "In situation X, when Y happens, the system must Z (measurable)."*

| # | Quality attribute | Scenario (situation → requirement) | Measurement / verification |
|---|-------------------|-------------------------------------|----------------------------|
| QS1 | [TBD] | | |

---

## 7. Solution Strategy

> *The architectural approach at a high level — the "how" before the "what".*

### 7.1 Cross-cutting Concepts (concrete)

> *Elaborate §2.5: name each cross-cutting concept and how it is realized.*

- [TBD]

### 7.2 Runtime View (strategy level)

> *Which major runtime interactions exist? (e.g. "user edits diagram → model updates → renderer produces ASCII text"). Diagram:*

```
[diagram placeholder — e.g. mermaid or ASCII]
```

### 7.3 Deployment View (strategy level)

> *Where and how does the system run? (static site? SPA with API? CLI companion?) High level only; details in §10.*

- [TBD]

### 7.4 Development View (strategy level)

> *How is the code organized at top level? (packages/modules, frontend vs. shared core vs. tests). Details in §8.*

- [TBD]

---

## 8. Building Block View

> *Static decomposition of the system, top-down. Level 0 = whole system; each level zooms into one building block from the previous level.*

### 8.1 Level 0: System Overview

> *One box for the whole system with its external interfaces (user input, output, external services).*

```
+--------------------------------------+
|        ascii-diagram-drawer          |
|                                      |
|  [inputs] -> [core] -> [outputs]     |
+--------------------------------------+
```

### 8.2 Level 1: Major Components

> *The 3–7 top-level components/modules and their interfaces.*

| Building block | Purpose | Interfaces (in/out) |
|----------------|---------|---------------------|
| [TBD] | | |

Diagram:

```
[placeholder]
```

### 8.3 Level 2: Component Internals

> *Zoom into each Level-1 component: sub-modules, data structures, key algorithms.*

#### 8.3.1 [Component A]

- Sub-modules: [TBD]
- Key data structures: [TBD]
- Key algorithms: [TBD]

*(Repeat for each Level-1 component.)*

### 8.4 Level 3: Module Internals (as needed)

> *Optional — only where complexity demands it (e.g. the layout/box-drawing algorithm).*

- [TBD]

---

## 9. Runtime View

> *How the system behaves at runtime for the key scenarios.*

### 9.1 Key Scenarios

> *For each scenario: name, trigger, sequence of steps across components (numbered list or sequence diagram).*

#### RS1: [TBD] (e.g. "Create and render a new diagram")

1. [TBD]
2. [TBD]
3. [TBD]

### 9.2 Additional Runtime Aspects

> *Concurrency, async behavior, state management, error flows.*

- [TBD]

---

## 10. Deployment View

> *How the system is deployed and operated.*

### 10.1 Deployment Concepts

> *Static hosting? Docker? npm package? CLI? List deployment targets.*

- [TBD]

### 10.2 Network Topology

> *Client/server boundaries, APIs called, external services (fonts, CDNs).*

```
[placeholder]
```

### 10.3 Deployment Model

> *Build pipeline: build → test → package → publish. CI tooling.*

- [TBD]

### 10.4 Operation & Maintenance

> *Monitoring, logging, updates, backups (as applicable).*

- [TBD]

### 10.5 Release Strategy

> *Versioning scheme, release cadence, deprecation policy.*

- [TBD]

---

## 11. Cross-cutting Concerns

> *Concerns that apply across building blocks.*

| Concern | Approach |
|---------|----------|
| Error handling | [TBD] |
| Logging / observability | [TBD] |
| Security (XSS, input validation) | [TBD] |
| Configuration | [TBD] |
| Internationalization / Unicode | [TBD] |
| Accessibility | [TBD] |

---

## 12. Concepts

> *Important concepts with stable identities — the vocabulary of the architecture (e.g. "DiagramModel", "Cell grid", "Box-drawing renderer").*

### Concept C1: [TBD]

- **ID:** C1
- **Purpose:** [TBD]
- **Properties / invariants:** [TBD]
- **Interfaces (provided/required):** [TBD]
- **Dependencies:** [TBD]

*(Repeat per concept.)*

---

## 13. Building Blocks (Detail)

> *Detailed specification of the important building blocks identified in §8.*

### BB1: [TBD]

- **ID:** BB1
- **Purpose:** [TBD]
- **Properties / non-functional requirements:** [TBD]
- **Interfaces (provided/required):** [TBD]
- **Dependencies:** [TBD]
- **Implementation notes:** [TBD]

*(Repeat per building block.)*

---

## 14. Identification of Important Requirements

> *The subset of functional and non-functional requirements that the architecture must satisfy — traceable to quality goals.*
>
> *arc42 convention: this section is a **selection**, not the complete requirements catalog. The full set of requirements is managed in: [TBD — e.g. separate spec doc, issue tracker]. Only architecturally relevant requirements are listed here.*

### 14.1 Functional Requirements

| ID | Requirement | Source (feature/user) | Priority |
|----|-------------|------------------------|----------|
| FR1 | [TBD] | | |

### 14.2 Non-functional Requirements

| ID | Requirement | Quality goal ref | Priority |
|----|-------------|------------------|----------|
| NFR1 | [TBD] | Q? | |

---

## 15. Risks and Technical Risks

> *Risk register: what could go wrong, impact, likelihood, mitigation/contingency.*

| ID | Risk description | Impact (H/M/L) | Likelihood (H/M/L) | Mitigation / contingency | Owner |
|----|------------------|----------------|--------------------|--------------------------|-------|
| R1 | [TBD] | | | | |

---

## 16. Glossary

> *Definitions of project-specific terms, so the document (and code) share one vocabulary.*

| Term | Definition |
|------|------------|
| [TBD] | [TBD] |
