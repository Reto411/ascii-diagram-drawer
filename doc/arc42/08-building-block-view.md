---
title: "arc42-8-building-block-view"
description: static decomposition of the system, top-down — level 0 to 3 building blocks
scope: architecture
status: draft
updated: 2026-10-17
---

# 8. Building Block View

> *Static decomposition of the system, top-down. Level 0 = whole system; each level zooms into one building block from the previous level.*

## 8.1 Level 0: System Overview

> *One box for the whole system with its external interfaces (user input, output, external services).*

```
+--------------------------------------+
|        ascii-diagram-drawer          |
|                                      |
|  [inputs] -> [core] -> [outputs]     |
+--------------------------------------+
```

## 8.2 Level 1: Major Components

> *The 3–7 top-level components/modules and their interfaces.*

| Building block | Purpose | Interfaces (in/out) |
|----------------|---------|---------------------|
| [TBD] | | |

Diagram:

```
[placeholder]
```

## 8.3 Level 2: Component Internals

> *Zoom into each Level-1 component: sub-modules, data structures, key algorithms.*

#### 8.3.1 [Component A]

- Sub-modules: [TBD]
- Key data structures: [TBD]
- Key algorithms: [TBD]

*(Repeat for each Level-1 component.)*

## 8.4 Level 3: Module Internals (as needed)

> *Optional — only where complexity demands it (e.g. the layout/box-drawing algorithm).*

- [TBD]
