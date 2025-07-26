use crate::{Heightmap, SLOWDOWN};
use lyon::tessellation::{self as tes, GeometryBuilder};
use nannou::prelude::*;
use nannou::{event::Update, glam::Vec2};
use tes::StrokeTessellator;

pub static MIN_DISTANCE: f32 = 15.0;
pub static POINT_SPACING: f32 = 5.0;

#[derive(Copy, Clone, Debug, Default)]
pub struct Node {
    pub loc: Vec2,
    pub tangent: Vec2,
    pub bitangent: Vec2,
    pub color: LinSrgba,
}

impl Node {
    pub fn step(&mut self, update: Update, heightmap: &Heightmap) {
        let up = heightmap.get(self.loc + vec2(1.0, 0.0));
        let down = heightmap.get(self.loc + vec2(-1.0, 0.0));
        let left = heightmap.get(self.loc + vec2(0.0, -1.0));
        let right = heightmap.get(self.loc + vec2(0.0, 1.0));
        let grad = -vec2(up - down, right - left);
        //self.loc += (self.tangent * 0.0 + -self.bitangent * 0.0 + grad * 35.0)
        self.loc += (self.tangent * 3.0 + -self.bitangent * 7.0 + grad * 35.0)
            * (update.since_last.as_secs_f32() - SLOWDOWN)
            * 3.0;
    }

    pub fn lyonize(&self, width: f32) -> (lyon::path::math::Point, impl AsRef<[f32]>) {
        (
            tes::geom::point(self.loc.x, self.loc.y),
            [
                width,
                self.color.red,
                self.color.green,
                self.color.blue,
                self.color.alpha,
            ],
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct River {
    pub start: Node,
    pub segments: Vec<Node>,
    pub end: Node,
    pub closed: bool,
    pub river_builder: RiverMeshBuilder,
}

impl River {
    pub fn node(&self, i: isize) -> Option<Node> {
        if i > 0 {
            self.segments.get(i as usize).copied()
        } else {
            None
        }
    }

    pub fn distribute(&mut self) {
        let mut new_nodes = Vec::<Node>::new();
        let mut at_loc = self.start.loc;
        let mut at_ind = 0;
        let mut distance_to_next_point = POINT_SPACING;
        let collision_distance = MIN_DISTANCE + 0.1;
        dbg!(self.segments.len());
        // dbg!(self.segments.iter().zip(.());
        while at_ind < self.segments.len() {
            let next_ind = self
                .segments
                .iter()
                .enumerate()
                .skip(at_ind + 1)
                .rev()
                .find_map(|(other_ind, other_node)| {
                    let ind_diff = at_ind.abs_diff(other_ind);
                    let close_margin = (MIN_DISTANCE / POINT_SPACING).ceil() as usize * 2;
                    if ind_diff < close_margin {
                        None
                    } else if (other_node.loc - at_loc).length_squared()
                        < collision_distance * collision_distance
                    {
                        Some(other_ind)
                    } else {
                        None
                    }
                })
                .unwrap_or(at_ind + 1);
            let next_node = self.node(next_ind as isize).unwrap_or(self.end);
            let mut line = next_node.loc - at_loc;
            let mut still_to_go = line.length();
            line /= still_to_go;
            while still_to_go > 0.01 {
                let step_by = distance_to_next_point.min(still_to_go);
                let moved = line * step_by;
                at_loc += moved;
                still_to_go -= step_by;
                distance_to_next_point -= step_by;
                if distance_to_next_point <= 0.0 {
                    new_nodes.push(Node {
                        loc: at_loc,
                        tangent: vec2(f32::NAN, f32::NAN),
                        bitangent: vec2(f32::NAN, f32::NAN),
                        color: next_node.color,
                    });
                    distance_to_next_point = POINT_SPACING;
                }
            }
            at_ind = next_ind;
        }

        self.segments = new_nodes;
    }

    pub fn recompute(&mut self) {
        let mut average = 0.0;
        for i in 0..self.segments.len() {
            let (a, b, c) = (
                self.node(i as isize - 1).unwrap_or(self.start),
                self.segments[i],
                self.node(i as isize + 1).unwrap_or(self.end),
            );
            average += b.loc.distance(a.loc);
            let (tangent, cross) = (
                (c.loc - a.loc).normalize_or_zero(),
                (b.loc - a.loc)
                    .normalize_or_zero()
                    .perp_dot((c.loc - b.loc).normalize_or_zero()),
            );
            self.segments[i].tangent = tangent;
            self.segments[i].bitangent = (tangent.perp() * cross.signum()).normalize_or_zero();
        }
        dbg!(average / self.segments.len() as f32);
    }

    pub fn step(&mut self, update: Update, heightmap: &Heightmap) {
        for node in &mut self.segments {
            node.step(update, heightmap);
        }
    }

    pub fn tesselate(&mut self, widthmap: &Heightmap) {
        self.river_builder.abort_geometry();

        let getwidth = |p| widthmap.get(p) * 10.0 + 15.0;
        let mut path_builder = lyon::path::Path::builder_with_attributes(5);
        {
            let (p, a) = self.start.lyonize(getwidth(self.start.loc));
            path_builder.begin(p, a.as_ref());
        }
        for p in &self.segments {
            let (p, a) = p.lyonize(getwidth(p.loc));
            path_builder.line_to(p, a.as_ref());
        }
        {
            let (p, a) = self.end.lyonize(getwidth(self.end.loc));
            path_builder.line_to(p, a.as_ref());
        }
        path_builder.end(self.closed);
        let path = path_builder.build();

        {
            let mut tessellator = StrokeTessellator::new();
            let mut opts = tes::StrokeOptions::default();
            opts.variable_line_width = Some(0);
            tessellator
                .tessellate_path(&path, &opts, &mut self.river_builder)
                .unwrap();
        }
    }

    pub fn draw_fill(&self, draw: &Draw) {
        draw.mesh()
            .indexed_colored(
                self.river_builder.vertices.iter().copied(),
                self.river_builder.indicies.iter().copied(),
            )
            .finish();
    }

    pub fn draw_for_history(&self, draw: &Draw) {
        draw.mesh()
            .indexed(
                self.river_builder.vertices.iter().map(|p| p.0),
                self.river_builder.indicies.iter().copied(),
            )
            .hsl(random(), 0.7, 0.2)
            .finish();
    }

    pub fn draw_border(&self, draw: &Draw) {
        let line = draw.polyline().weight(2.0).color(BLACK);
        if self.closed {
            line.points(self.river_builder.left_bank.iter().map(|p| p.1));
        } else {
            line.points(self.river_builder.left_bank.iter().map(|p| p.1));
        }
        let line = draw.polyline().weight(2.0).color(BLACK);
        if self.closed {
            line.points(self.river_builder.right_bank.iter().map(|p| p.1));
        } else {
            line.points(self.river_builder.right_bank.iter().map(|p| p.1));
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct RiverMeshBuilder {
    vertices: Vec<(Vec3, LinSrgba)>,
    indicies: Vec<usize>,
    left_bank: Vec<(f32, Vec2)>,
    right_bank: Vec<(f32, Vec2)>,
}

impl tes::GeometryBuilder for RiverMeshBuilder {
    fn add_triangle(&mut self, a: tes::VertexId, b: tes::VertexId, c: tes::VertexId) {
        self.indicies.push(a.to_usize());
        self.indicies.push(b.to_usize());
        self.indicies.push(c.to_usize());
    }

    fn begin_geometry(&mut self) {}

    fn end_geometry(&mut self) {
        self.left_bank
            .sort_by(|(t_a, _), (t_b, _)| t_a.partial_cmp(t_b).unwrap());
        self.right_bank
            .sort_by(|(t_a, _), (t_b, _)| t_a.partial_cmp(t_b).unwrap());
    }

    fn abort_geometry(&mut self) {
        self.vertices.clear();
        self.indicies.clear();
        self.left_bank.clear();
        self.right_bank.clear();
    }
}

impl tes::StrokeGeometryBuilder for RiverMeshBuilder {
    fn add_stroke_vertex(
        &mut self,
        mut vertex: tes::StrokeVertex,
    ) -> Result<tes::VertexId, tes::GeometryBuilderError> {
        let t = vertex.advancement();
        let pos = vertex.position();
        let pos = vec2(pos.x, pos.y);
        match vertex.side() {
            tes::Side::Positive => {
                self.left_bank.push((t, pos));
            }
            tes::Side::Negative => {
                self.right_bank.push((t, pos));
            }
        }
        let i = self.vertices.len() as u32;
        let p = vec3(vertex.position().x, vertex.position().y, 0.0);
        let a = vertex.interpolated_attributes();
        self.vertices.push((p, lin_srgba(a[1], a[2], a[3], a[4])));
        Ok(tes::VertexId(i))
    }
}
