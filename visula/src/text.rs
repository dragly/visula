use crate::pipelines::polygons::PolygonVertex;
use lyon::math::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers};

#[derive(Default)]
pub struct TessellatedGeometry {
    pub vertices: Vec<PolygonVertex>,
    pub indices: Vec<u32>,
}

impl TessellatedGeometry {
    pub fn merge(&mut self, other: &TessellatedGeometry) {
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

pub struct WrappedText {
    pub geometry: TessellatedGeometry,
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
    let metrics = face.metrics();
    let lines = face.wrap_lines(text, max_width);
    let mut geometry = TessellatedGeometry::default();
    let mut width: f32 = 0.0;
    for (index, line) in lines.iter().enumerate() {
        let baseline_y = -(metrics.ascent + metrics.line_height * index as f32);
        width = width.max(face.tessellate_line(line, baseline_y, &mut geometry));
    }
    Ok(WrappedText {
        geometry,
        width,
        height: metrics.line_height * lines.len() as f32,
    })
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
        let mut cursor_x: f32 = 0.0;
        for ch in line.chars() {
            let Some(glyph_id) = self.face.glyph_index(ch) else {
                continue;
            };
            let mut outline = GlyphOutlineCollector::new(self.scale_factor, cursor_x, baseline_y);
            if self.face.outline_glyph(glyph_id, &mut outline).is_some() {
                if let Ok(glyph) = tessellate_path(&outline.build()) {
                    geometry.merge(&glyph);
                }
            }
            cursor_x += self.advance(glyph_id);
        }
        cursor_x
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
}
