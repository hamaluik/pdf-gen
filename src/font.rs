use crate::{
    refs::{ObjectReferences, RefType},
    PDFError, Pt,
};
use id_arena::Id;
use owned_ttf_parser::{AsFaceRef, OwnedFace};
use pdf_writer::{
    types::{FontFlags, SystemInfo},
    Finish, Name, Pdf, Ref, Str,
};
use std::collections::HashMap;

/// A parsed font object. Fonts can be TTF or OTF fonts, and will be embedded in their
/// entirety in the generated PDF, so large fonts may dramatically increase the size of
/// the generated PDF. Future versions will explore subsetting the fonts.
///
/// Currently, font lifetimes _must_ exceed document lifetimes in order to be properly
/// embedded. This may change in the future.
///
/// Typically, fonts are referred to throughout user applications by their _index_ within
/// the document itself, and not by any typed references
pub struct Font {
    pub face: OwnedFace,
}

impl Font {
    /// Load a font from raw bytes, parsing the font and returning an error if the font
    /// could not be parsed
    pub fn load(bytes: Vec<u8>) -> Result<Font, PDFError> {
        let face = OwnedFace::from_vec(bytes, 0)?;

        Ok(Font { face })
    }

    /// Obtain the full name of the font. Panics if the font does not have a name
    pub fn name(&self) -> String {
        self.face
            .as_face_ref()
            .names()
            .into_iter()
            .find(|name| name.name_id == owned_ttf_parser::name_id::FULL_NAME && name.is_unicode())
            .and_then(|name| name.to_string())
            .expect("font face has a name")
    }

    /// Obtain the family name of the font. Panics if the font does not have a font family
    pub fn family(&self) -> String {
        self.face
            .as_face_ref()
            .names()
            .into_iter()
            .find(|name| name.name_id == owned_ttf_parser::name_id::FAMILY && name.is_unicode())
            .and_then(|name| name.to_string())
            .expect("font face has a family")
    }

    /// Calculate the ascent (distance from the baseline to the top of the font) for the given font size
    pub fn ascent(&self, size: Pt) -> Pt {
        let scaling: Pt = size / self.face.as_face_ref().units_per_em() as f32;
        scaling * self.face.as_face_ref().ascender() as f32
    }

    /// Calculate the descent (distance from the baseline to the bottom of the font) for the given font size.
    /// Note: this is usually negative
    pub fn descent(&self, size: Pt) -> Pt {
        let scaling: Pt = size / self.face.as_face_ref().units_per_em() as f32;
        scaling * self.face.as_face_ref().descender() as f32
    }

    /// Calculate the leading (extra space between lines) for the given font size
    pub fn leading(&self, size: Pt) -> Pt {
        let scaling: Pt = size / self.face.as_face_ref().units_per_em() as f32;
        scaling * self.face.as_face_ref().line_gap() as f32
    }

    /// Calculate the default line height of the font for the given size. The returned value is
    /// how much to vertically offset a second row of text below a first row of text.
    pub fn line_height(&self, size: Pt) -> Pt {
        let scaling: Pt = size / self.face.as_face_ref().units_per_em() as f32;
        let leading: Pt = scaling * self.face.as_face_ref().line_gap() as f32;
        let ascent: Pt = scaling * self.face.as_face_ref().ascender() as f32;
        let descent: Pt = scaling * self.face.as_face_ref().descender() as f32;
        leading + ascent - descent
    }

    /// Obtain the weight of the font. Numerical values generally map as follows:
    ///
    /// * 100: Thin (Hairline)
    /// * 200: Extra Light (Ultra Light)
    /// * 300: Light
    /// * 400: Normal
    /// * 500: Medium
    /// * 600: Semi Bold (Demi Bold)
    /// * 700: Bold
    /// * 800: Extra Bold (Ultra Bold)
    /// * 900: Black (Heavy)
    pub fn weight(&self) -> u16 {
        self.face.as_face_ref().weight().to_number()
    }

    fn write_cid(&self, refs: &mut ObjectReferences, font_index: usize, writer: &mut Pdf) -> Ref {
        let font_descriptor_id = self.write_descriptor(refs, font_index, writer);

        let id = refs.gen(RefType::CidFont(font_index));

        let mut cid_font = writer.cid_font(id);
        cid_font.subtype(pdf_writer::types::CidFontType::Type2);
        cid_font.base_font(Name(format!("F{font_index}").as_bytes()));
        cid_font.system_info(SystemInfo {
            registry: Str(b"Adobe"),
            ordering: Str(b"Identity"),
            supplement: 0,
        });
        cid_font.font_descriptor(font_descriptor_id);

        let ids = self.glyph_ids();
        let ids_augmented = self.glyphs_sizing(&ids);

        let scaling = 1000.0 / self.face.as_face_ref().units_per_em() as f32;

        // find the most popular width to use as the default
        // <width, count>
        let mut widths_counts: HashMap<u16, usize> = HashMap::new();
        for (_, (_, (width, _))) in ids_augmented.iter() {
            *widths_counts.entry(*width).or_insert(0) += 1;
        }
        let most_common_width = widths_counts
            .iter()
            .max_by_key(|(&sz, _)| sz)
            .map(|(&sz, _)| sz as f32 * scaling)
            .unwrap_or(1000.0);

        let mut widths = cid_font.widths();
        widths.consecutive(0, [1000.0]);

        let mut id_widths: Vec<(u16, f32)> = ids_augmented
            .iter()
            .map(|(&cid, &(_, (width, _)))| (cid, width as f32 * scaling))
            .collect();
        id_widths.sort_by_key(|(id, _)| *id);

        // TODO: compress with ranges as well
        let first = id_widths.first().expect("font has at least 1 glyph in it");
        let mut start_cid: u16 = first.0;
        let mut current_widths: Vec<f32> = vec![first.1];
        for (cid, width) in id_widths.into_iter().skip(1) {
            if (cid - start_cid) as usize > current_widths.len() {
                // we need a new block!
                widths.consecutive(start_cid, current_widths.clone());
                start_cid = cid;
                current_widths.clear();
            }

            current_widths.push(width);
        }

        if !current_widths.is_empty() {
            widths.consecutive(start_cid, current_widths);
        }

        widths.finish();

        cid_font.default_width(most_common_width);
        cid_font.cid_to_gid_map_predefined(Name(b"Identity"));

        id
    }

    fn write_font_data(
        &self,
        refs: &mut ObjectReferences,
        font_index: usize,
        writer: &mut Pdf,
    ) -> Ref {
        let id = refs.gen(RefType::FontData(font_index));

        writer
            .stream(id, self.face.as_slice())
            .pair(Name(b"Length1"), self.face.as_slice().len() as i32);

        id
    }

    fn write_descriptor(
        &self,
        refs: &mut ObjectReferences,
        font_index: usize,
        writer: &mut Pdf,
    ) -> Ref {
        let font_data_stream_id = self.write_font_data(refs, font_index, writer);

        let gids = self.glyph_ids();
        let gids_augmented = self.glyphs_sizing(&gids);

        let max_width = gids_augmented
            .values()
            .map(|&(_, (w, _))| w)
            .max()
            .unwrap_or_default();
        let max_height = gids_augmented
            .values()
            .map(|&(_, (_, h))| h)
            .max()
            .unwrap_or_default();
        let sum_width: usize = gids_augmented.values().map(|&(_, (w, _))| w as usize).sum();
        let avg_width = sum_width as f32 / gids_augmented.len() as f32;

        let id = refs.gen(RefType::FontDescriptor(font_index));

        let mut descriptor = writer.font_descriptor(id);
        descriptor.name(Name(self.name().as_bytes()));
        descriptor.family(Str(self.family().as_bytes()));
        descriptor.weight(self.face.as_face_ref().weight().to_number());

        let mut flags: FontFlags = FontFlags::empty();
        if self.face.as_face_ref().is_monospaced() {
            flags.set(FontFlags::FIXED_PITCH, true);
        }
        if self.face.as_face_ref().is_italic() {
            flags.set(FontFlags::ITALIC, true);
        }
        descriptor.flags(flags);

        let scaling = 1000.0 / self.face.as_face_ref().units_per_em() as f32;
        descriptor.bbox(pdf_writer::Rect {
            x1: 0.0,
            y1: 0.0,
            x2: sum_width as f32 * scaling,
            y2: max_height as f32 * scaling,
        });
        descriptor.italic_angle(self.face.as_face_ref().italic_angle().unwrap_or_default());
        descriptor.ascent(self.face.as_face_ref().ascender() as f32 * scaling);
        descriptor.descent(self.face.as_face_ref().descender() as f32 * scaling);
        descriptor.leading(self.face.as_face_ref().line_gap() as f32 * scaling);
        descriptor.cap_height(
            self.face
                .as_face_ref()
                .capital_height()
                .map(|h| h as f32 * scaling)
                .unwrap_or(1000.0),
        );
        descriptor.x_height(
            self.face
                .as_face_ref()
                .x_height()
                .unwrap_or_else(|| self.face.as_face_ref().capital_height().unwrap_or_default())
                as f32
                * scaling,
        );
        //descriptor.stem_v(todo!());
        // TODO: how to get this?
        descriptor.stem_v(80.0);
        //descriptor.stem_h(todo!());
        descriptor.avg_width(avg_width * scaling);
        descriptor.max_width(max_width as f32 * scaling);
        descriptor.missing_width(max_width as f32 * scaling);

        descriptor.font_file2(font_data_stream_id);

        id
    }

    fn glyph_ids(&self) -> HashMap<u16, char> {
        // Adapted from printpdf
        let mut map: HashMap<u16, char> = HashMap::new();

        for subtable in self
            .face
            .as_face_ref()
            .tables()
            .cmap
            .expect("font has cmap table")
            .subtables
            .into_iter()
            .filter(|table| table.is_unicode())
        {
            subtable.codepoints(|codepoint: u32| {
                if let Ok(ch) = char::try_from(codepoint) {
                    if let Some(index) = subtable.glyph_index(codepoint).filter(|index| index.0 > 0)
                    {
                        map.entry(index.0).or_insert(ch);
                    }
                }
            });
        }

        map
    }

    fn glyphs_sizing(&self, ids: &HashMap<u16, char>) -> HashMap<u16, (char, (u16, i16))> {
        let mut ids_augmented: HashMap<u16, (char, (u16, i16))> = HashMap::new();
        for (&id, &ch) in ids.iter() {
            if let Some(gid) = self.face.as_face_ref().glyph_index(ch) {
                if let Some(h_advance) = self.face.as_face_ref().glyph_hor_advance(gid) {
                    let height = self
                        .face
                        .as_face_ref()
                        .glyph_bounding_box(gid)
                        .map(|bbox| bbox.y_max - bbox.y_min - self.face.as_face_ref().descender())
                        .unwrap_or(1000);
                    ids_augmented.insert(id, (ch, (h_advance, height)));
                }
            }
        }
        ids_augmented
    }

    fn write_to_unicode(
        &self,
        refs: &mut ObjectReferences,
        font_index: usize,
        writer: &mut Pdf,
    ) -> Ref {
        let id = refs.gen(RefType::ToUnicode(font_index));

        let mut map: String = r#"/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
/Ordering (UCS) /Supplement 0 >> def
/CMapName /Adobe-Identity-UCS def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
"#
        .replace("\r\n", "\n");

        let ids = self.glyph_ids();
        let mut ids: Vec<(u16, char)> = ids.into_iter().collect();
        ids.sort_by_key(|&(id, _)| id);

        // segment the cmap into appropriate segments
        // each segment has a maximum length of 100
        // each segment has a common high byte
        let mut cmap_blocks: Vec<Vec<(u16, char)>> = Vec::new();
        let mut current_block: Vec<(u16, char)> = Vec::new();
        let mut high_byte: u8 = 0;
        for (id, ch) in ids.iter() {
            if (id >> 8) as u8 != high_byte || current_block.len() >= 100 {
                cmap_blocks.push(current_block.clone());
                current_block.clear();
                high_byte = (id >> 8) as u8;
            }

            current_block.push((*id, *ch));
        }
        if !current_block.is_empty() {
            cmap_blocks.push(current_block);
        }

        for block in cmap_blocks.into_iter() {
            map.push_str(&format!("{} beginbfchar\n", block.len()));
            for (id, ch) in block.into_iter() {
                let ch: u32 = ch.into();
                map.push_str(&format!("<{id:04x}> <{:04x}>\n", ch));
            }
            map.push_str("endbfchar\n");
        }

        map.push_str("endcmap CMapName currentdict /CMap defineresource pop end end\n");

        let compressed = miniz_oxide::deflate::compress_to_vec_zlib(
            map.as_bytes(),
            miniz_oxide::deflate::CompressionLevel::DefaultCompression as u8,
        );
        let mut stream = writer.stream(id, compressed.as_slice());
        stream.filter(pdf_writer::Filter::FlateDecode);

        id
    }

    pub(crate) fn write(&self, refs: &mut ObjectReferences, id: Id<Font>, writer: &mut Pdf) {
        let font_index = id.index();
        let font_id = refs.gen(RefType::Font(font_index));
        let cid_font_id = self.write_cid(refs, font_index, writer);
        let to_unicode_id = self.write_to_unicode(refs, font_index, writer);

        let mut font = writer.type0_font(font_id);
        font.base_font(Name(format!("F{font_index}").as_bytes()));
        font.encoding_predefined(Name(b"Identity-H"));
        font.descendant_font(cid_font_id);
        font.to_unicode(to_unicode_id);
    }

    pub fn glyph_id(&self, ch: char) -> Option<u16> {
        self.face.as_face_ref().glyph_index(ch).map(|i| i.0)
    }

    pub fn replacement_glyph_id(&self) -> Option<u16> {
        self.face.as_face_ref().glyph_index('\u{FFFD}').map(|i| i.0)
    }
}
