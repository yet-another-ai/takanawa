---
layout: home

hero:
  name: Takanawa
  text: Resilient range downloads for Rust and native SDKs.
  tagline: Ship resumable downloads across desktop, Android, and Apple platforms with a small Rust core and stable native interfaces.
  actions:
    - theme: brand
      text: Get Started
      link: /guide/getting-started
    - theme: alt
      text: .part Format
      link: /part-format

features:
  - title: Crash-safe resume state
    details: Dual metadata slots keep partially completed downloads recoverable after interrupted writes.
  - title: Rust core, native surfaces
    details: Use the core crates directly or ship through C ABI, Android AAR, and SwiftPM artifacts.
  - title: Range-aware HTTP engine
    details: Chunk planning and HTTP range requests are designed for predictable recovery and verification.
---
