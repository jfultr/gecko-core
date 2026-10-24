# Gecko and Axis

Version 0.7 · October 10, 2026 · [Русский](model.ru.md)

[Glossary](../0-concepts/index.md) · [Mesh / App](../1-mesh-app/model.ru.md) · [Line / Point / Edge / View](../4-line-point-edge-view/model.md) · [Block / Anchor](../3-block-anchor/model.md)

## 1. Gecko

The user starts Gecko instances on available machines and platforms. Each Gecko instance discovers accessible Devices, exposes discovered or explicitly connected Cameras, and advertises Axes, known processing implementations, and connected services. Gecko instances that can reach one another form a Mesh; supported connection checks provide bandwidth and latency measurements.

“The Gecko project” refers to the whole system, while “a Gecko instance” refers to a particular runtime. A machine provides the environment for Gecko. This document does not constrain the number of instances on one machine.

## 2. Axes and Execution Requirements

**Gecko advertises Axes.** A Device describes particular hardware; an Axis describes an available environmental capability considering hardware, software, and access. Points describe the local requirements of their selected implementations. A Line’s own requirements and those of its Points and internal Edges must all hold; placement validation considers the whole Line. If Gecko prepares a Block provider, that implementation’s requirements are checked separately at its execution location. A Line’s domain requirements for a Block are described by Anchors, not Axes.

```text
Gecko jetson-01 advertises: camera, gstreamer, tensorrt
Gecko desktop-01 advertises: ui.slider, ui.video_view

Inference Point requires: a supported inference backend
Line camera_front requires: capabilities for all its Points
```

Requirements belong to a particular implementation. An Inference Point with a local model and an Inference Point using a Block through an Anchor may have different local requirements. A remote Block does not turn its GPU into a local Axis of another Gecko instance.

Capability and current resource availability are distinct: having TensorRT does not confirm free GPU memory, and having a camera does not confirm access at launch. Resources, Anchor conditions, and Edge support are checked separately.

Hardware is described in the [Devices section](../0-concepts/index.md#4-devices).

## 3. Vocabulary Changes

### From Version 0.5 to 0.6

| Previously | Now |
|---|---|
| App selection and capability presentation are assigned to Admin / Editor | Gecko provides these capabilities regardless of interaction mechanism |

### Retained Changes in Version 0.2 from 0.1

The Kubernetes comparison retains its original meaning: Gecko understands media/AI processing, supported connections, and domain dependencies. Kubernetes can provide an execution environment for part of the system. This map does not establish a direct correspondence between a Line or Block and a Pod.
