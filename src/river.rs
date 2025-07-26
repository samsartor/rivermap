use crate::{Heightmap, SLOWDOWN};
use lyon::path::traits::PathBuilder;
use lyon::tessellation as tes;
use nannou::draw::mesh::MeshBuilder;
use nannou::draw::mesh::builder::{ColorPerPoint, SingleColor};
use nannou::draw::{Drawing, Mesh};
use nannou::prelude::*;
use nannou::{event::Update, glam::Vec2};
use std::iter::once;
use tes::{GeometryBuilder, StrokeTessellator};

pub static MIN_DISTANCE: f32 = 5.0;

#[derive(Copy, Clone, Debug, Default)]
pub struct Node {
    pub loc: Vec2,
    pub tangent: Vec2,
    pub bitangent: Vec2,
    pub color: LinSrgba,
    pub width: f32,
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

    pub fn lyonize<T>(&self, func: impl FnOnce(lyon::path::math::Point, &[f32]) -> T) -> T {
        func(
            tes::geom::point(self.loc.x, self.loc.y),
            &[1.0, 0.0, 0.0, 1.0, 10.0],
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct River {
    pub start: Node,
    pub segments: Vec<Node>,
    pub end: Node,
    pub closed: bool,
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
        let mut distance_to_next_point = MIN_DISTANCE;
        let collision_distance = MIN_DISTANCE + 0.1;
        while at_ind < self.segments.len() {
            let next_ind = self
                .segments
                .iter()
                .enumerate()
                .skip(at_ind + 1)
                .rev()
                .find_map(|(other_ind, other_node)| {
                    if (other_node.loc - at_loc).length_squared()
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
                        width: next_node.width,
                    });
                    distance_to_next_point = MIN_DISTANCE;
                }
            }
            at_ind = next_ind;
        }

        self.segments = new_nodes;
    }

    pub fn draw(&self, draw: &Draw) {
        let segments = self.segments.iter().copied();
        let points = once(self.start)
            .chain(segments)
            .chain(once(self.end))
            .map(|Node { loc, .. }| (loc, PINK));
        let line = draw.polyline().weight(MIN_DISTANCE);
        if self.closed {
            line.points_colored_closed(points);
        } else {
            line.points_colored(points);
        }
        // Does calling sleep(0.0) still trigger os stuff?
        if SLOWDOWN != 0.0 {
            std::thread::sleep(std::time::Duration::from_secs_f32(SLOWDOWN));
        }
    }

    pub fn recompute(&mut self) {
        for i in 0..self.segments.len() {
            let (a, b, c) = (
                self.node(i as isize - 1).unwrap_or(self.start),
                self.segments[i],
                self.node(i as isize + 1).unwrap_or(self.end),
            );
            let (tangent, cross) = (
                (c.loc - a.loc).normalize_or_zero(),
                (b.loc - a.loc)
                    .normalize_or_zero()
                    .perp_dot((c.loc - b.loc).normalize_or_zero()),
            );
            self.segments[i].tangent = tangent;
            self.segments[i].bitangent = (tangent.perp() * cross.signum()).normalize_or_zero();
        }
    }

    pub fn step(&mut self, update: Update, heightmap: &Heightmap) {
        for node in &mut self.segments {
            node.step(update, heightmap);
        }
    }

    pub fn draw_dumb(&self, draw: &Draw) {
        let mut river_builder = RiverMeshBuilder {
            vertices: Vec::new(),
            indicies: Vec::new(),
            left_bank: Vec::new(),
            right_bank: Vec::new(),
        };

        let mut path_builder = lyon::path::Path::builder_with_attributes(5);
        self.start.lyonize(|p, a| path_builder.begin(p, a));
        for p in &self.segments {
            p.lyonize(|p, a| {
                path_builder.line_to(p, a);
            });
        }
        self.end.lyonize(|p, a| {
            path_builder.line_to(p, a);
        });
        path_builder.end(self.closed);
        let path = path_builder.build();

        {
            let mut tessellator = StrokeTessellator::new();
            let mut opts = tes::StrokeOptions::default();
            opts.variable_line_width = Some(4);
            tessellator
                .tessellate_path(&path, &opts, &mut river_builder)
                .unwrap();
        }

        draw.mesh()
            .indexed_colored(river_builder.vertices, river_builder.indicies)
            .finish();
    }
}

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
        self.vertices.push((p, lin_srgba(a[0], a[1], a[2], a[3])));
        Ok(tes::VertexId(i))
    }
}
