use crate::pipelines::glyphs::GlyphVertex;
use crate::pipelines::polygons::PolygonVertex;
use fdsm::bezier::scanline::FillRule;
use fdsm::bezier::{Point, Segment};
use fdsm::correct_error::{correct_error_msdf, ErrorCorrectionConfig};
use fdsm::generate::generate_msdf;
use fdsm::render::correct_sign_msdf;
use fdsm::shape::{Contour, Shape};
use image::Rgb32FImage;
use lyon::math::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers};
use std::collections::HashMap;
use std::sync::Arc;

/// The distance in atlas texels between a glyph field's 0 and 1, centered on the outline.
pub const DISTANCE_RANGE: f32 = 6.0;
const PIXELS_PER_EM: f32 = 48.0;
const GLYPH_PADDING: f32 = 4.0;
const GLYPH_GAP: u32 = 1;
const ATLAS_WIDTH: u32 = 1024;
const INITIAL_ATLAS_HEIGHT: u32 = 256;
const CORNER_SIN_ALPHA: f64 = 0.03;
const EDGE_COLORING_SEED: u64 = 69441337420;

pub struct TessellatedGeometry<V = PolygonVertex> {
    pub vertices: Vec<V>,
    pub indices: Vec<u32>,
}

impl<V> Default for TessellatedGeometry<V> {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }
}

impl<V: Clone> TessellatedGeometry<V> {
    pub fn merge(&mut self, other: &TessellatedGeometry<V>) {
        let base_index = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&other.vertices);
        self.indices
            .extend(other.indices.iter().map(|i| i + base_index));
    }
}

pub struct FontMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub line_height: f32,
}

pub struct WrappedText<V = PolygonVertex> {
    pub geometry: TessellatedGeometry<V>,
    pub width: f32,
    pub height: f32,
}

pub fn tessellate_text(
    font_data: &[u8],
    text: &str,
    scale: f32,
) -> Result<TessellatedGeometry, crate::error::Error> {
    let face = ScaledFace::parse(font_data, scale)?;
    let mut geometry = TessellatedGeometry::default();
    face.tessellate_line(text, 0.0, &mut geometry);
    Ok(geometry)
}

pub fn tessellate_text_wrapped(
    font_data: &[u8],
    text: &str,
    scale: f32,
    max_width: f32,
) -> Result<WrappedText, crate::error::Error> {
    let face = ScaledFace::parse(font_data, scale)?;
    Ok(face.wrap(text, max_width, |line, baseline_y, geometry| {
        face.tessellate_line(line, baseline_y, geometry)
    }))
}

pub fn measure_text_width(
    font_data: &[u8],
    text: &str,
    scale: f32,
) -> Result<f32, crate::error::Error> {
    Ok(ScaledFace::parse(font_data, scale)?.width(text))
}

pub fn font_metrics(font_data: &[u8], scale: f32) -> Result<FontMetrics, crate::error::Error> {
    Ok(ScaledFace::parse(font_data, scale)?.metrics())
}

struct ScaledFace<'a> {
    face: ttf_parser::Face<'a>,
    scale_factor: f32,
}

impl<'a> ScaledFace<'a> {
    fn parse(font_data: &'a [u8], scale: f32) -> Result<Self, crate::error::Error> {
        let face =
            ttf_parser::Face::parse(font_data, 0).map_err(|_| crate::error::Error::FontParse)?;
        let scale_factor = scale / face.units_per_em() as f32;
        Ok(Self { face, scale_factor })
    }

    fn metrics(&self) -> FontMetrics {
        let ascent = self.face.ascender() as f32 * self.scale_factor;
        let descent = self.face.descender() as f32 * self.scale_factor;
        let line_gap = self.face.line_gap() as f32 * self.scale_factor;
        FontMetrics {
            ascent,
            descent,
            line_height: ascent - descent + line_gap,
        }
    }

    fn advance(&self, glyph_id: ttf_parser::GlyphId) -> f32 {
        self.face.glyph_hor_advance(glyph_id).unwrap_or(0) as f32 * self.scale_factor
    }

    fn width(&self, text: &str) -> f32 {
        text.chars()
            .filter_map(|ch| self.face.glyph_index(ch))
            .map(|glyph_id| self.advance(glyph_id))
            .sum()
    }

    fn glyphs<'b>(
        &'b self,
        line: &'b str,
    ) -> impl Iterator<Item = (ttf_parser::GlyphId, f32)> + 'b {
        line.chars()
            .filter_map(move |ch| self.face.glyph_index(ch))
            .scan(0.0, move |cursor_x, glyph_id| {
                let x = *cursor_x;
                *cursor_x += self.advance(glyph_id);
                Some((glyph_id, x))
            })
    }

    fn wrap<V>(
        &self,
        text: &str,
        max_width: f32,
        mut lay_out_line: impl FnMut(&str, f32, &mut TessellatedGeometry<V>) -> f32,
    ) -> WrappedText<V> {
        let metrics = self.metrics();
        let lines = self.wrap_lines(text, max_width);
        let mut geometry = TessellatedGeometry::default();
        let mut width: f32 = 0.0;
        for (index, line) in lines.iter().enumerate() {
            let baseline_y = -(metrics.ascent + metrics.line_height * index as f32);
            width = width.max(lay_out_line(line, baseline_y, &mut geometry));
        }
        WrappedText {
            geometry,
            width,
            height: metrics.line_height * lines.len() as f32,
        }
    }

    fn wrap_lines(&self, text: &str, max_width: f32) -> Vec<String> {
        let space_width = self.width(" ");
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
            let mut line = String::new();
            let mut line_width = 0.0;
            for word in paragraph.split_whitespace() {
                let word_width = self.width(word);
                if !line.is_empty() && line_width + space_width + word_width > max_width {
                    lines.push(std::mem::take(&mut line));
                    line_width = 0.0;
                }
                if !line.is_empty() {
                    line.push(' ');
                    line_width += space_width;
                }
                line.push_str(word);
                line_width += word_width;
            }
            lines.push(line);
        }
        lines
    }

    fn tessellate_line(
        &self,
        line: &str,
        baseline_y: f32,
        geometry: &mut TessellatedGeometry,
    ) -> f32 {
        for (glyph_id, cursor_x) in self.glyphs(line) {
            let mut outline = GlyphOutlineCollector::new(self.scale_factor, cursor_x, baseline_y);
            if self.face.outline_glyph(glyph_id, &mut outline).is_some() {
                if let Ok(glyph) = tessellate_path(&outline.build()) {
                    geometry.merge(&glyph);
                }
            }
        }
        self.width(line)
    }
}

/// Where a glyph's distance field lies in the atlas, and where its corner lies
/// relative to the glyph's origin, both in texels.
#[derive(Clone, Copy, Debug)]
struct AtlasGlyph {
    texel: [f32; 2],
    size: [f32; 2],
    offset: [f32; 2],
}

/// An RGBA texture of multi-channel signed distance fields for the glyphs of one font,
/// filled in as text is laid out. Draw the text with [`crate::Glyphs`].
pub struct GlyphAtlas {
    font_data: Arc<[u8]>,
    glyphs: HashMap<ttf_parser::GlyphId, Option<AtlasGlyph>>,
    pixels: Vec<u8>,
    height: u32,
    shelf: [u32; 3],
    revision: u64,
}

impl GlyphAtlas {
    pub fn new(font_data: &[u8]) -> Result<Self, crate::error::Error> {
        ttf_parser::Face::parse(font_data, 0).map_err(|_| crate::error::Error::FontParse)?;
        Ok(Self {
            font_data: font_data.into(),
            glyphs: HashMap::new(),
            pixels: vec![0; (4 * ATLAS_WIDTH * INITIAL_ATLAS_HEIGHT) as usize],
            height: INITIAL_ATLAS_HEIGHT,
            shelf: [0; 3],
            revision: 0,
        })
    }

    pub fn width(&self) -> u32 {
        ATLAS_WIDTH
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Changes whenever glyphs are added to the atlas.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Lays out text like [`tessellate_text_wrapped`], as one quad per glyph,
    /// and adds the glyphs it has not seen before to the atlas.
    pub fn lay_out_wrapped(
        &mut self,
        text: &str,
        scale: f32,
        max_width: f32,
    ) -> Result<WrappedText<GlyphVertex>, crate::error::Error> {
        let font_data = self.font_data.clone();
        let face = ScaledFace::parse(&font_data, scale)?;
        let world_per_texel = scale / PIXELS_PER_EM;
        Ok(face.wrap(text, max_width, |line, baseline_y, geometry| {
            for (glyph_id, cursor_x) in face.glyphs(line) {
                let Some(glyph) = self.glyph(&face.face, glyph_id) else {
                    continue;
                };
                let left = cursor_x + glyph.offset[0] * world_per_texel;
                let bottom = baseline_y + glyph.offset[1] * world_per_texel;
                let [width, height] = glyph.size.map(|size| size * world_per_texel);
                let [u, v] = glyph.texel;
                let [du, dv] = glyph.size;
                let first = geometry.vertices.len() as u32;
                geometry.vertices.extend([
                    GlyphVertex {
                        position: [left, bottom],
                        texel: [u, v],
                    },
                    GlyphVertex {
                        position: [left + width, bottom],
                        texel: [u + du, v],
                    },
                    GlyphVertex {
                        position: [left + width, bottom + height],
                        texel: [u + du, v + dv],
                    },
                    GlyphVertex {
                        position: [left, bottom + height],
                        texel: [u, v + dv],
                    },
                ]);
                geometry
                    .indices
                    .extend([0, 1, 2, 0, 2, 3].map(|corner| first + corner));
            }
            face.width(line)
        }))
    }

    fn glyph(
        &mut self,
        face: &ttf_parser::Face,
        glyph_id: ttf_parser::GlyphId,
    ) -> Option<AtlasGlyph> {
        if let Some(glyph) = self.glyphs.get(&glyph_id) {
            return *glyph;
        }
        let glyph = distance_field(face, glyph_id).map(|(field, offset)| {
            let texel = self.place(&field);
            AtlasGlyph {
                texel: texel.map(|t| t as f32),
                size: [field.width() as f32, field.height() as f32],
                offset,
            }
        });
        self.glyphs.insert(glyph_id, glyph);
        glyph
    }

    fn place(&mut self, field: &Rgb32FImage) -> [u32; 2] {
        let [mut x, mut y, mut row_height] = self.shelf;
        if x + field.width() > ATLAS_WIDTH {
            [x, y, row_height] = [0, y + row_height + GLYPH_GAP, 0];
        }
        while y + field.height() > self.height {
            self.height *= 2;
            self.pixels
                .resize((4 * ATLAS_WIDTH * self.height) as usize, 0);
        }
        for (column, row, pixel) in field.enumerate_pixels() {
            let start = (4 * ((y + row) * ATLAS_WIDTH + x + column)) as usize;
            let [r, g, b] = pixel
                .0
                .map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8);
            self.pixels[start..start + 4].copy_from_slice(&[r, g, b, 255]);
        }
        self.shelf = [
            x + field.width() + GLYPH_GAP,
            y,
            row_height.max(field.height()),
        ];
        self.revision += 1;
        [x, y]
    }
}

/// The multi-channel signed distance field of a glyph with y pointing up,
/// along with the offset of its corner from the glyph's origin in texels.
fn distance_field(
    face: &ttf_parser::Face,
    glyph_id: ttf_parser::GlyphId,
) -> Option<(Rgb32FImage, [f32; 2])> {
    let bounds = face.glyph_bounding_box(glyph_id)?;
    let texels_per_unit = PIXELS_PER_EM / face.units_per_em() as f32;
    let left = (bounds.x_min as f32 * texels_per_unit).floor() - GLYPH_PADDING;
    let bottom = (bounds.y_min as f32 * texels_per_unit).floor() - GLYPH_PADDING;
    let right = (bounds.x_max as f32 * texels_per_unit).ceil() + GLYPH_PADDING;
    let top = (bounds.y_max as f32 * texels_per_unit).ceil() + GLYPH_PADDING;
    let mut outline = ShapeCollector {
        scale: texels_per_unit as f64,
        offset: [-left as f64, -bottom as f64],
        shape: Shape::default(),
        contour: Contour::default(),
        start: Point::origin(),
        current: Point::origin(),
    };
    face.outline_glyph(glyph_id, &mut outline)?;
    let shape = Shape::edge_coloring_simple(outline.shape, CORNER_SIN_ALPHA, EDGE_COLORING_SEED);
    let prepared = shape.prepare();
    let range = DISTANCE_RANGE as f64;
    let mut field = Rgb32FImage::new((right - left) as u32, (top - bottom) as u32);
    generate_msdf(&prepared, range, &mut field);
    correct_error_msdf(
        &mut field,
        &shape,
        &prepared,
        range,
        &ErrorCorrectionConfig::default(),
    );
    correct_sign_msdf(&mut field, &prepared, FillRule::Nonzero);
    Some((field, [left, bottom]))
}

struct ShapeCollector {
    scale: f64,
    offset: [f64; 2],
    shape: Shape<Contour>,
    contour: Contour,
    start: Point,
    current: Point,
}

impl ShapeCollector {
    fn point(&self, x: f32, y: f32) -> Point {
        Point::new(
            x as f64 * self.scale + self.offset[0],
            y as f64 * self.scale + self.offset[1],
        )
    }

    fn push(&mut self, segment: Segment, end: Point) {
        self.contour.segments.push(segment);
        self.current = end;
    }
}

impl ttf_parser::OutlineBuilder for ShapeCollector {
    fn move_to(&mut self, x: f32, y: f32) {
        self.start = self.point(x, y);
        self.current = self.start;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let end = self.point(x, y);
        if end != self.current {
            self.push(Segment::line(self.current, end), end);
        }
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let end = self.point(x, y);
        self.push(Segment::quad(self.current, self.point(x1, y1), end), end);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let end = self.point(x, y);
        let control = [self.point(x1, y1), self.point(x2, y2)];
        self.push(
            Segment::cubic(self.current, control[0], control[1], end),
            end,
        );
    }

    fn close(&mut self) {
        if self.current != self.start {
            self.push(Segment::line(self.current, self.start), self.start);
        }
        let contour = std::mem::take(&mut self.contour);
        if !contour.segments.is_empty() {
            self.shape.contours.push(contour);
        }
    }
}

pub fn tessellate_path(path: &Path) -> Result<TessellatedGeometry, crate::error::Error> {
    let mut geometry: VertexBuffers<PolygonVertex, u32> = VertexBuffers::new();
    let mut tessellator = FillTessellator::new();
    tessellator.tessellate_path(
        path,
        &FillOptions::tolerance(0.01),
        &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| PolygonVertex {
            position: [vertex.position().x, vertex.position().y],
        }),
    )?;

    Ok(TessellatedGeometry {
        vertices: geometry.vertices,
        indices: geometry.indices,
    })
}

pub fn tessellate_regular_polygon(
    cx: f32,
    cy: f32,
    radius: f32,
    sides: u32,
) -> Result<TessellatedGeometry, crate::error::Error> {
    let mut builder = Path::builder();
    for i in 0..sides {
        let angle = std::f32::consts::TAU * i as f32 / sides as f32 - std::f32::consts::FRAC_PI_2;
        let x = cx + radius * angle.cos();
        let y = cy + radius * angle.sin();
        if i == 0 {
            builder.begin(point(x, y));
        } else {
            builder.line_to(point(x, y));
        }
    }
    builder.close();
    let path = builder.build();
    tessellate_path(&path)
}

pub fn tessellate_star(
    cx: f32,
    cy: f32,
    outer_radius: f32,
    inner_radius: f32,
    points: u32,
) -> Result<TessellatedGeometry, crate::error::Error> {
    let mut builder = Path::builder();
    let total = points * 2;
    for i in 0..total {
        let angle = std::f32::consts::TAU * i as f32 / total as f32 - std::f32::consts::FRAC_PI_2;
        let r = if i % 2 == 0 {
            outer_radius
        } else {
            inner_radius
        };
        let x = cx + r * angle.cos();
        let y = cy + r * angle.sin();
        if i == 0 {
            builder.begin(point(x, y));
        } else {
            builder.line_to(point(x, y));
        }
    }
    builder.close();
    let path = builder.build();
    tessellate_path(&path)
}

struct GlyphOutlineCollector {
    builder: lyon::path::path::Builder,
    scale: f32,
    offset_x: f32,
    offset_y: f32,
}

impl GlyphOutlineCollector {
    fn new(scale: f32, offset_x: f32, offset_y: f32) -> Self {
        Self {
            builder: Path::builder(),
            scale,
            offset_x,
            offset_y,
        }
    }

    fn build(self) -> Path {
        self.builder.build()
    }
}

impl ttf_parser::OutlineBuilder for GlyphOutlineCollector {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder.begin(point(
            x * self.scale + self.offset_x,
            y * self.scale + self.offset_y,
        ));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.line_to(point(
            x * self.scale + self.offset_x,
            y * self.scale + self.offset_y,
        ));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.builder.quadratic_bezier_to(
            point(
                x1 * self.scale + self.offset_x,
                y1 * self.scale + self.offset_y,
            ),
            point(
                x * self.scale + self.offset_x,
                y * self.scale + self.offset_y,
            ),
        );
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.builder.cubic_bezier_to(
            point(
                x1 * self.scale + self.offset_x,
                y1 * self.scale + self.offset_y,
            ),
            point(
                x2 * self.scale + self.offset_x,
                y2 * self.scale + self.offset_y,
            ),
            point(
                x * self.scale + self.offset_x,
                y * self.scale + self.offset_y,
            ),
        );
    }

    fn close(&mut self) {
        self.builder.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font() -> Vec<u8> {
        egui::FontDefinitions::default().font_data["Ubuntu-Light"]
            .font
            .to_vec()
    }

    #[test]
    fn wraps_at_word_boundaries() {
        let font = font();
        let face = ScaledFace::parse(&font, 1.0).unwrap();
        let max_width = (face.width("one two") + face.width("one two three")) / 2.0;
        assert_eq!(
            face.wrap_lines("one  two three\n\nfour", max_width),
            ["one two", "three", "", "four"]
        );
    }

    #[test]
    fn wrapped_text_fits_its_bounds() {
        let font = font();
        let line_height = font_metrics(&font, 2.0).unwrap().line_height;
        let wrapped = tessellate_text_wrapped(&font, "Hello wrapped world", 2.0, 1.0).unwrap();
        assert_eq!(wrapped.height, 3.0 * line_height);
        assert!(!wrapped.geometry.indices.is_empty());
        for vertex in &wrapped.geometry.vertices {
            let [x, y] = vertex.position;
            assert!((0.0..=wrapped.width).contains(&x));
            assert!((-wrapped.height..=0.0).contains(&y));
        }
    }

    fn bounds(positions: impl Iterator<Item = [f32; 2]>) -> [[f32; 2]; 2] {
        positions.fold([[f32::MAX; 2], [f32::MIN; 2]], |[min, max], [x, y]| {
            [
                [min[0].min(x), min[1].min(y)],
                [max[0].max(x), max[1].max(y)],
            ]
        })
    }

    fn median_at(atlas: &GlyphAtlas, [u, v]: [f32; 2]) -> u8 {
        let start = 4 * (v as usize * atlas.width() as usize + u as usize);
        let mut channels = [
            atlas.pixels()[start],
            atlas.pixels()[start + 1],
            atlas.pixels()[start + 2],
        ];
        channels.sort();
        channels[1]
    }

    #[test]
    fn lays_out_one_quad_per_visible_glyph() {
        let mut atlas = GlyphAtlas::new(&font()).unwrap();
        let wrapped = atlas.lay_out_wrapped("Hi there", 2.0, 100.0).unwrap();
        assert_eq!(wrapped.geometry.vertices.len(), 4 * 7);
        assert_eq!(wrapped.geometry.indices.len(), 6 * 7);
        let tessellated = tessellate_text_wrapped(&font(), "Hi there", 2.0, 100.0).unwrap();
        assert_eq!(wrapped.width, tessellated.width);
        assert_eq!(wrapped.height, tessellated.height);
    }

    #[test]
    fn glyph_quads_cover_the_outlines_with_a_margin() {
        let scale = 2.0;
        let mut atlas = GlyphAtlas::new(&font()).unwrap();
        let quads = atlas.lay_out_wrapped("o", scale, 100.0).unwrap();
        let outline = tessellate_text_wrapped(&font(), "o", scale, 100.0).unwrap();
        let [quad_min, quad_max] = bounds(quads.geometry.vertices.iter().map(|v| v.position));
        let [outline_min, outline_max] =
            bounds(outline.geometry.vertices.iter().map(|v| v.position));
        let world_per_texel = scale / PIXELS_PER_EM;
        for axis in 0..2 {
            let below = outline_min[axis] - quad_min[axis];
            let above = quad_max[axis] - outline_max[axis];
            for margin in [below, above] {
                assert!(margin >= GLYPH_PADDING * world_per_texel);
                assert!(margin <= (GLYPH_PADDING + 1.0) * world_per_texel);
            }
        }
    }

    #[test]
    fn the_field_is_inside_the_glyph_and_outside_its_corners() {
        let mut atlas = GlyphAtlas::new(&font()).unwrap();
        let quad = atlas
            .lay_out_wrapped("I", 1.0, 100.0)
            .unwrap()
            .geometry
            .vertices;
        let [min, max] = bounds(quad.iter().map(|v| v.texel));
        let center = [(min[0] + max[0]) / 2.0, (min[1] + max[1]) / 2.0];
        assert!(median_at(&atlas, center) > 150);
        for corner in [min, [max[0] - 1.0, max[1] - 1.0]] {
            assert!(median_at(&atlas, corner) < 50);
        }
    }

    #[test]
    fn reuses_glyphs_already_in_the_atlas() {
        let mut atlas = GlyphAtlas::new(&font()).unwrap();
        let first = atlas.lay_out_wrapped("aa", 1.0, 100.0).unwrap();
        let revision = atlas.revision();
        assert_eq!(revision, 1);
        let second = atlas.lay_out_wrapped("a", 3.0, 100.0).unwrap();
        assert_eq!(atlas.revision(), revision);
        let texels = |text: &WrappedText<GlyphVertex>, quad: usize| {
            text.geometry.vertices[4 * quad..4 * quad + 4]
                .iter()
                .map(|v| v.texel)
                .collect::<Vec<_>>()
        };
        assert_eq!(texels(&first, 0), texels(&first, 1));
        assert_eq!(texels(&first, 0), texels(&second, 0));
    }

    #[test]
    fn grows_to_fit_every_glyph() {
        let mut atlas = GlyphAtlas::new(&font()).unwrap();
        let text: String = (' '..='~').chain('¡'..='ÿ').collect();
        let wrapped = atlas.lay_out_wrapped(&text, 1.0, 100.0).unwrap();
        assert!(atlas.height() > INITIAL_ATLAS_HEIGHT);
        assert_eq!(
            atlas.pixels().len(),
            (4 * atlas.width() * atlas.height()) as usize
        );
        let [min, max] = bounds(wrapped.geometry.vertices.iter().map(|v| v.texel));
        assert!(min[0] >= 0.0 && min[1] >= 0.0);
        assert!(max[0] <= atlas.width() as f32 && max[1] <= atlas.height() as f32);
    }
}
