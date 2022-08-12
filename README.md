# pdf-gen
A mid-level, opionated library for generating PDF documents

[![Crates.io](https://img.shields.io/crates/v/pdf-gen.svg)](https://crates.io/crates/pdf-gen) ![license](https://img.shields.io/crates/l/pdf-gen) ![maintenance](https://img.shields.io/badge/maintenance-passively--maintained-yellowgreen.svg)

---

A small library for abstracting some of the core PDF generation logic away, built on top of [pdf-writer](https://crates.io/crates/pdf-writer)

Current Features:

* Unicode font embedding
* Raster and SVG image embedding
* Page generation with laid out text spans, images, or raw PDF contents
* Document metadata
* Compressed streams where possible
* Basic text layout utilities
