# pdf-gen
A mid-level, opinionated library for generating PDF documents

[![Crates.io](https://img.shields.io/crates/v/pdf-gen.svg)](https://crates.io/crates/pdf-gen) ![license](https://img.shields.io/crates/l/pdf-gen) ![maintenance](https://img.shields.io/badge/maintenance-passively--maintained-yellowgreen.svg)

---

A small library for abstracting some of the core PDF generation logic away, built on top of [pdf-writer](https://crates.io/crates/pdf-writer)

Current Features:

* Unicode font embedding
* Raster and SVG image embedding
* Page generation with laid out text spans, images, or raw PDF contents
* Form XObjects for reusable content with transformation support (scale, rotate, translate)
* Document bookmarks and intra-document navigation links
* Document metadata
* Compressed streams where possible
* Basic text layout utilities

## Form XObjects

Form XObjects allow you to define reusable content that can be placed multiple times
with different transformations. This is useful for scenarios like:

- Booklet imposition (placing multiple logical pages on a single physical sheet)
- Repeated content like logos or watermarks
- Content that needs transformation (rotation, scaling)

```rust
use pdf_gen::{Document, Page, FormXObject, FormXObjectLayout, Transform, Pt};
use pdf_gen::pagesize;

let mut doc = Document::default();

// create a form xobject with some content
let mut form = FormXObject::new(Pt(100.0), Pt(50.0));
// add content to form (spans, images, raw content)...

let form_id = doc.add_form_xobject(form);

// place the form on a page with a transformation
let mut page = Page::new(pagesize::LETTER, None);
page.add_form_xobject(FormXObjectLayout {
    xobj_id: form_id,
    transform: Transform::translate(Pt(72.0), Pt(72.0)),
});

doc.add_page(page);
```
