use clap::Parser;
use glam::{Vec3, Vec4};
use lyon::math::point;
use lyon::path::Path as LyonPath;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
};
use std::path::PathBuf;
use usvg::{Node, Paint, Tree};
use visula::{
    CustomEvent, Expression, PolygonDelegate, PolygonVertex, Polygons, RenderData, Renderable,
};
use winit::event::Event;

#[derive(Parser, Debug)]
#[command(about = "Render an SVG file using Visula")]
struct Args {
    #[arg(value_name = "SVG_FILE")]
    svg_path: PathBuf,
}

#[derive(Debug)]
struct Error {}

struct RenderedPath {
    polygons: Polygons,
}

fn usvg_color_to_vec4(color: &usvg::Color, opacity: f32) -> Vec4 {
    Vec4::new(
        color.red as f32 / 255.0,
        color.green as f32 / 255.0,
        color.blue as f32 / 255.0,
        opacity,
    )
}

fn tiny_skia_path_to_lyon(
    path: &usvg::tiny_skia_path::Path,
    transform: usvg::Transform,
) -> LyonPath {
    let mut builder = LyonPath::builder();
    let mut has_open_subpath = false;
    for seg in path.segments() {
        match seg {
            usvg::tiny_skia_path::PathSegment::MoveTo(p) => {
                if has_open_subpath {
                    builder.end(false);
                }
                let (tx, ty) = transform_point(p.x, p.y, &transform);
                builder.begin(point(tx, ty));
                has_open_subpath = true;
            }
            usvg::tiny_skia_path::PathSegment::LineTo(p) => {
                let (tx, ty) = transform_point(p.x, p.y, &transform);
                builder.line_to(point(tx, ty));
            }
            usvg::tiny_skia_path::PathSegment::QuadTo(p1, p) => {
                let (tx1, ty1) = transform_point(p1.x, p1.y, &transform);
                let (tx, ty) = transform_point(p.x, p.y, &transform);
                builder.quadratic_bezier_to(point(tx1, ty1), point(tx, ty));
            }
            usvg::tiny_skia_path::PathSegment::CubicTo(p1, p2, p) => {
                let (tx1, ty1) = transform_point(p1.x, p1.y, &transform);
                let (tx2, ty2) = transform_point(p2.x, p2.y, &transform);
                let (tx, ty) = transform_point(p.x, p.y, &transform);
                builder.cubic_bezier_to(point(tx1, ty1), point(tx2, ty2), point(tx, ty));
            }
            usvg::tiny_skia_path::PathSegment::Close => {
                builder.close();
                has_open_subpath = false;
            }
        }
    }
    if has_open_subpath {
        builder.end(false);
    }
    builder.build()
}

fn transform_point(x: f32, y: f32, t: &usvg::Transform) -> (f32, f32) {
    let tx = t.sx * x + t.kx * y + t.tx;
    let ty = t.ky * x + t.sy * y + t.ty;
    (tx, -ty)
}

fn tessellate_fill(lyon_path: &LyonPath) -> Option<(Vec<PolygonVertex>, Vec<u32>)> {
    let mut geometry: VertexBuffers<PolygonVertex, u32> = VertexBuffers::new();
    let mut tessellator = FillTessellator::new();
    tessellator
        .tessellate_path(
            lyon_path,
            &FillOptions::tolerance(0.01),
            &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| PolygonVertex {
                position: [vertex.position().x, vertex.position().y],
            }),
        )
        .ok()?;
    if geometry.vertices.is_empty() {
        return None;
    }
    Some((geometry.vertices, geometry.indices))
}

fn tessellate_stroke(lyon_path: &LyonPath, width: f32) -> Option<(Vec<PolygonVertex>, Vec<u32>)> {
    let mut geometry: VertexBuffers<PolygonVertex, u32> = VertexBuffers::new();
    let mut tessellator = StrokeTessellator::new();
    tessellator
        .tessellate_path(
            lyon_path,
            &StrokeOptions::tolerance(0.01).with_line_width(width),
            &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| PolygonVertex {
                position: [vertex.position().x, vertex.position().y],
            }),
        )
        .ok()?;
    if geometry.vertices.is_empty() {
        return None;
    }
    Some((geometry.vertices, geometry.indices))
}

struct Simulation {
    rendered_paths: Vec<RenderedPath>,
}

impl Simulation {
    fn new(application: &mut visula::Application, svg_path: &PathBuf) -> Result<Self, Error> {
        let svg_data = std::fs::read(svg_path).expect("Failed to read SVG file");
        let tree =
            Tree::from_data(&svg_data, &usvg::Options::default()).expect("Failed to parse SVG");

        let svg_size = tree.size();
        let scale = 10.0 / svg_size.width().max(svg_size.height());
        let center_x = svg_size.width() * scale / 2.0;
        let center_y = svg_size.height() * scale / 2.0;

        application.camera_controller.current_transform.center = Vec3::new(0.0, 0.0, 0.0);
        application.camera_controller.current_transform.forward = Vec3::new(0.0, 0.0, -1.0);
        application.camera_controller.current_transform.up = Vec3::new(0.0, 1.0, 0.0);
        application.camera_controller.current_transform.distance = 12.0;
        application.camera_controller.target_transform =
            application.camera_controller.current_transform.clone();

        let mut rendered_paths = Vec::new();

        let root = tree.root();
        let scale_transform =
            usvg::Transform::from_row(scale, 0.0, 0.0, scale, -center_x, -center_y);
        let root_transform = root.transform();
        let combined = scale_transform.post_concat(root_transform);

        let mut depth_index: u32 = 0;
        for child in root.children() {
            process_node_with_scale(
                child,
                application,
                &mut rendered_paths,
                combined,
                &mut depth_index,
            );
        }

        Ok(Simulation { rendered_paths })
    }
}

const DEPTH_STEP: f32 = 0.001;

fn process_node_with_scale(
    node: &Node,
    application: &mut visula::Application,
    rendered_paths: &mut Vec<RenderedPath>,
    parent_scale: usvg::Transform,
    depth_index: &mut u32,
) {
    match node {
        Node::Path(path) => {
            if !path.is_visible() {
                return;
            }
            let transform = parent_scale.pre_concat(path.abs_transform());
            let lyon_path = tiny_skia_path_to_lyon(path.data(), transform);

            if let Some(fill) = path.fill() {
                let color = match fill.paint() {
                    Paint::Color(c) => usvg_color_to_vec4(c, fill.opacity().get()),
                    _ => Vec4::new(0.5, 0.5, 0.5, fill.opacity().get()),
                };
                let z = *depth_index as f32 * DEPTH_STEP;
                *depth_index += 1;
                if let Some((vertices, indices)) = tessellate_fill(&lyon_path) {
                    if let Ok(polygons) = Polygons::new(
                        &application.rendering_descriptor(),
                        &PolygonDelegate {
                            color: Expression::from(color),
                            position: Vec3::new(0.0, 0.0, z).into(),
                        },
                        &vertices,
                        &indices,
                    ) {
                        rendered_paths.push(RenderedPath { polygons });
                    }
                }
            }

            if let Some(stroke) = path.stroke() {
                let color = match stroke.paint() {
                    Paint::Color(c) => usvg_color_to_vec4(c, stroke.opacity().get()),
                    _ => Vec4::new(0.5, 0.5, 0.5, stroke.opacity().get()),
                };
                let z = *depth_index as f32 * DEPTH_STEP;
                *depth_index += 1;
                let width = stroke.width().get() * parent_scale.sx.abs();
                if let Some((vertices, indices)) = tessellate_stroke(&lyon_path, width) {
                    if let Ok(polygons) = Polygons::new(
                        &application.rendering_descriptor(),
                        &PolygonDelegate {
                            color: Expression::from(color),
                            position: Vec3::new(0.0, 0.0, z).into(),
                        },
                        &vertices,
                        &indices,
                    ) {
                        rendered_paths.push(RenderedPath { polygons });
                    }
                }
            }
        }
        Node::Group(group) => {
            for child in group.children() {
                process_node_with_scale(
                    child,
                    application,
                    rendered_paths,
                    parent_scale,
                    depth_index,
                );
            }
        }
        Node::Text(text) => {
            let group = text.flattened();
            for child in group.children() {
                process_node_with_scale(
                    child,
                    application,
                    rendered_paths,
                    parent_scale,
                    depth_index,
                );
            }
        }
        Node::Image(_) => {}
    }
}

impl visula::Simulation for Simulation {
    type Error = Error;

    fn render(&mut self, data: &mut RenderData) {
        for path in &self.rendered_paths {
            path.polygons.render(data);
        }
    }

    fn handle_event(
        &mut self,
        _application: &mut visula::Application,
        _event: &Event<CustomEvent>,
    ) {
    }
}

fn main() -> Result<(), visula::error::Error> {
    let args = Args::parse();
    let svg_path = args.svg_path.clone();
    visula::run(move |app| Simulation::new(app, &svg_path))
}
