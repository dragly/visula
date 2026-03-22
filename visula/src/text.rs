use crate::pipelines::polygons::PolygonVertex;
use lyon::math::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers};

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

pub fn tessellate_text(
    font_data: &[u8],
    text: &str,
    scale: f32,
) -> Result<TessellatedGeometry, crate::error::Error> {
    let face = ttf_parser::Face::parse(font_data, 0).map_err(|_| crate::error::Error::FontParse)?;
    let units_per_em = face.units_per_em() as f32;
    let scale_factor = scale / units_per_em;

    let mut all_vertices = Vec::new();
    let mut all_indices = Vec::new();
    let mut cursor_x: f32 = 0.0;

    for ch in text.chars() {
        let glyph_id = match face.glyph_index(ch) {
            Some(id) => id,
            None => continue,
        };

        let mut outline = GlyphOutlineCollector::new(scale_factor, cursor_x, 0.0);
        if face.outline_glyph(glyph_id, &mut outline).is_some() {
            let path = outline.build();
            let mut geometry: VertexBuffers<PolygonVertex, u32> = VertexBuffers::new();
            let mut tessellator = FillTessellator::new();
            if tessellator
                .tessellate_path(
                    &path,
                    &FillOptions::tolerance(0.01),
                    &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| PolygonVertex {
                        position: [vertex.position().x, vertex.position().y],
                    }),
                )
                .is_ok()
            {
                let base_index = all_vertices.len() as u32;
                all_vertices.extend_from_slice(&geometry.vertices);
                all_indices.extend(geometry.indices.iter().map(|i| i + base_index));
            }
        }

        let advance = face.glyph_hor_advance(glyph_id).unwrap_or(0) as f32 * scale_factor;
        cursor_x += advance;
    }

    Ok(TessellatedGeometry {
        vertices: all_vertices,
        indices: all_indices,
    })
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
