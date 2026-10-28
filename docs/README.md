# Gecko Documentation

Start with the [goal and concepts](0-concepts/index.md). The [development cycle](development-cycle.md) records the current work route and return questions for the upward pass. The [module catalog](core-modules.md) lists accepted foundational modules and their dependencies.

English is the single documentation language. Documents use filenames without a language suffix; update related pages together when a rule or contract changes.

| Topic | Active document | Supporting documents |
|---|---|---|
| Concepts and goal | [Concept map](0-concepts/index.md) | [Development cycle and progress](development-cycle.md) |
| Mesh / App | [Model](1-mesh-app/model.md) | [Requirements on lower levels](1-mesh-app/lower-level-requirements.md), [revision history](1-mesh-app/history.md) |
| Gecko / Axis | [Model](2-gecko-axis/model.md) | [Scenario](2-gecko-axis/scenario.md), [D2 plan](2-gecko-axis/work-plan.md), [decision history](2-gecko-axis/history.md), [research](2-gecko-axis/researches/operations-and-reconnaissance.md) |
| Block / Anchor | [Model](3-block-anchor/model.md) | — |
| Line / Point / Edge / View | [Model](4-line-point-edge-view/model.md) | — |
| Foundational modules | [Catalog](core-modules.md) | [Operation module](../docs-modules/operation/README.md) |

Each topic's `model.md` contains its active rules and questions. Requirements, history, and working materials are separate from the model. Historical decisions do not override active rules. The root [development-cycle.md](development-cycle.md) describes work order and progress rather than a domain contract.

The [temporary Mesh / App plan](1-mesh-app/archive/work-plan.md) and [October 22, 2026 review](1-mesh-app/archive/review-2026-10-22.md) are historical materials from an earlier stage. Their status does not imply completion of the current cycle.

To validate local Markdown links and section anchors, run `python3 docs/check_links.py` from the repository root.
