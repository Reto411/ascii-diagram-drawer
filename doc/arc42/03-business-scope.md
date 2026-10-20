---
title: "arc42-3-business-scope"
description: product context, features, user characteristics — outside perspective, no technical language
scope: architecture
status: draft
updated: 2026-10-18
---

# 3. Business Scope

> *What the product is from an outside perspective — no technical language.*

## 3.1 Context

ascii-diagram-drawer is an **open-source visual editor for text-based diagrams**. It was started by a junior architect who got stuck with PlantUML: its output looks outdated, and its automatic layout engine takes control of the diagram — in complex systems the result becomes spaghetti that is hard to fix.

**Target audience:** anyone who needs to model complex systems — exactly the place where auto-layout tools start to fail. No prior ASCII-art skills are required.

**Where it fits in the user's workflow:**

- **Before:** the user today hand-writes PlantUML (or another text diagram language) and fights its layout engine, or draws ASCII by hand, or uses image-based tools whose results don't fit into text documents.
- **During:** the user places, moves, and resizes shapes on a canvas, connects them with arrows, and reuses or defines their own objects.
- **After:** the finished diagram is copied as plain text into READMEs, design docs, RFCs, pull requests, and chat — or exported as an image (SVG/PNG) for slides and web pages.
- **Over time:** diagrams are **maintained assets**. They live alongside the system and must be updated as the system evolves — so editing has to stay easy and the layout must not drift on its own between edits.
- **Later (v1+):** existing hand-typed or foreign ASCII diagrams can be brought back into the tool and edited visually (see F9).

## 3.2 Products and Features

| # | Feature | Description | As of |
|---|---------|-------------|-------|
| F1 | Visual canvas editor | Place, move, and resize predefined shapes (boxes, circles, …) on a grid-aligned canvas | v1 |
| F2 | Connectors & anchor points | Draw lines/arrows between objects; they snap to **anchor points** and stay connected when the object moves or is resized. Predefined shapes come with standard anchors (basic positions + official UML connection points); for your own shapes you define the anchor points yourself — in the shape file or directly in the editor; the set applies to all objects of that type | v1 |
| F3 | Plain-text output | The diagram *is* plain monospace text — pasteable into terminals, READMEs, design docs, chat, commit messages | v1 |
| F4 | Image export | SVG (vector) and PNG (raster) exports that always match the text representation exactly | v1 |
| F5 | Custom diagram objects | Define your own shapes entirely in plain text; anchor points are definable in the shape file or directly in the editor | v1 |
| F6 | Collections | Store objects in named collections for organization, reuse across diagrams, and sharing | v1 |
| F7 | Grid alignment & scaling | Shapes snap to a grid; one control scales the whole diagram up for more readable output | v1 |
| F8 | Cross-platform | One app on Windows, macOS, Linux, and in the browser | v1 |
| F9 | Import existing ASCII | Bring an existing ASCII diagram back into the tool: regions matching known shapes load as fully editable objects; unknown regions load with a warning in reduced form (no resizing/grouping) and can optionally be registered as new custom shapes | v1+ |

## 3.3 User Characteristics

- **Who:** architects and developers who document systems — design docs, RFCs, READMEs. Comfortable with monospace text and version control; **no** ASCII-art fluency expected.
- **Expectations:** full control over layout (nothing rearranges itself); fast iteration; output that looks good both as plain text and as an image; no lock-in — the files stay readable and editable outside the tool, forever.
- **Environment:** desktop (Windows/macOS/Linux) or browser; diagrams live in code repositories next to the documentation they describe.
- **Pain with the status quo:** auto-layout tools produce spaghetti on complex diagrams and offer little control; hand-typed ASCII is slow and fiddly to maintain.
