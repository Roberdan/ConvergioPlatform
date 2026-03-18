mod spatial;

use spatial::SpatialHash;
use wasm_bindgen::prelude::*;

const REPULSION_K: f32 = 90.0;
const REPULSION_RADIUS_MULTIPLIER: f32 = 3.5;
const REPULSION_FORCE_SCALE: f32 = 0.5;
const SPRING_FORCE: f32 = 0.002;
const SPRING_SESSION_MULTIPLIER: f32 = 2.2;
const SPRING_DEFAULT_MULTIPLIER: f32 = 1.4;
const GRAVITY_STRONG: f32 = 0.01;
const GRAVITY_WEAK: f32 = 0.0015;
const DAMPING: f32 = 0.82;
const BOUNDS_MARGIN_MIN: f32 = 20.0;
const BOUNDS_REBOUND: f32 = 0.3;
const GRID_SIZE: f32 = 50.0;
const MAX_REPULSION_FORCE: f32 = 3.0;

#[wasm_bindgen]
pub struct PhysicsEngine {
    nodes: Vec<Node>,
    synapses: Vec<Synapse>,
    spatial: SpatialHash,
    width: f32,
    height: f32,
    center_x: f32,
    center_y: f32,
}

#[derive(Clone)]
struct Node {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    radius: f32,
    mass: f32,
    pinned: bool,
    is_session: bool,
}

struct Synapse {
    from: usize,
    to: usize,
    strength: f32,
    spring_length: f32,
    is_session_link: bool,
}

#[wasm_bindgen]
impl PhysicsEngine {
    #[wasm_bindgen(constructor)]
    pub fn new(width: f32, height: f32) -> PhysicsEngine {
        PhysicsEngine {
            nodes: Vec::new(),
            synapses: Vec::new(),
            spatial: SpatialHash::new(GRID_SIZE),
            width,
            height,
            center_x: width * 0.5,
            center_y: height * 0.5,
        }
    }

    pub fn set_bounds(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.center_x = width * 0.5;
        self.center_y = height * 0.5;
    }

    pub fn set_nodes(&mut self, data: &[f32], count: usize) {
        self.nodes.clear();
        self.nodes.reserve(count);

        for i in 0..count {
            let base = i * 8;
            if base + 7 >= data.len() {
                break;
            }
            let radius = data[base + 4].max(1.0);
            let mass = data[base + 5].max(1.0);
            self.nodes.push(Node {
                x: data[base],
                y: data[base + 1],
                vx: data[base + 2],
                vy: data[base + 3],
                radius,
                mass,
                pinned: data[base + 6] > 0.5,
                is_session: data[base + 7] > 0.5,
            });
        }
    }

    pub fn set_synapses(&mut self, indices: &[u32], props: &[f32], count: usize) {
        self.synapses.clear();
        self.synapses.reserve(count);

        for i in 0..count {
            let idx_base = i * 2;
            let prop_base = i * 3;
            if idx_base + 1 >= indices.len() || prop_base + 2 >= props.len() {
                break;
            }
            self.synapses.push(Synapse {
                from: indices[idx_base] as usize,
                to: indices[idx_base + 1] as usize,
                strength: props[prop_base],
                spring_length: props[prop_base + 1],
                is_session_link: props[prop_base + 2] > 0.5,
            });
        }
    }

    pub fn step(&mut self) -> Vec<f32> {
        self.single_step();
        self.snapshot()
    }

    pub fn step_budget(&mut self, budget_ms: f32) -> Vec<f32> {
        let mut steps = (budget_ms / 2.0).floor() as usize;
        if steps == 0 {
            steps = 1;
        }
        let capped = steps.min(4);
        for _ in 0..capped {
            self.single_step();
        }
        self.snapshot()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl PhysicsEngine {
    fn single_step(&mut self) {
        let node_count = self.nodes.len();
        if node_count == 0 {
            return;
        }

        let repulsion_radius = REPULSION_K * REPULSION_RADIUS_MULTIPLIER;
        self.spatial.set_cell_size(repulsion_radius.max(GRID_SIZE));
        self.spatial.clear();
        for (i, n) in self.nodes.iter().enumerate() {
            self.spatial.insert(i, n.x, n.y);
        }

        for i in 0..node_count {
            let (x, y) = (self.nodes[i].x, self.nodes[i].y);
            let nearby = self.spatial.get_nearby(x, y, repulsion_radius);
            for &j in &nearby {
                if j <= i {
                    continue;
                }

                let dx = self.nodes[i].x - self.nodes[j].x;
                let dy = self.nodes[i].y - self.nodes[j].y;
                let d2 = dx * dx + dy * dy;
                if d2 <= f32::EPSILON {
                    continue;
                }
                let dist = d2.sqrt();
                if dist > repulsion_radius {
                    continue;
                }

                let mut f = (REPULSION_K * REPULSION_K) / d2 * REPULSION_FORCE_SCALE;
                if f > MAX_REPULSION_FORCE {
                    f = MAX_REPULSION_FORCE;
                }

                let fx = (dx / dist) * f;
                let fy = (dy / dist) * f;
                if !self.nodes[i].pinned {
                    self.nodes[i].vx += fx / self.nodes[i].mass;
                    self.nodes[i].vy += fy / self.nodes[i].mass;
                }
                if !self.nodes[j].pinned {
                    self.nodes[j].vx -= fx / self.nodes[j].mass;
                    self.nodes[j].vy -= fy / self.nodes[j].mass;
                }
            }
        }

        for syn in &self.synapses {
            if syn.from >= node_count || syn.to >= node_count || syn.from == syn.to {
                continue;
            }

            let (ax, ay) = (self.nodes[syn.from].x, self.nodes[syn.from].y);
            let (bx, by) = (self.nodes[syn.to].x, self.nodes[syn.to].y);
            let dx = bx - ax;
            let dy = by - ay;
            let d2 = dx * dx + dy * dy;
            if d2 <= f32::EPSILON {
                continue;
            }
            let dist = d2.sqrt();
            let session_multiplier = if syn.is_session_link {
                SPRING_SESSION_MULTIPLIER
            } else {
                SPRING_DEFAULT_MULTIPLIER
            };
            let rest = syn.spring_length * session_multiplier;
            let f = (dist - rest) * SPRING_FORCE * syn.strength.max(0.05);
            let fx = (dx / dist) * f;
            let fy = (dy / dist) * f;

            if !self.nodes[syn.from].pinned {
                self.nodes[syn.from].vx += fx / self.nodes[syn.from].mass;
                self.nodes[syn.from].vy += fy / self.nodes[syn.from].mass;
            }
            if !self.nodes[syn.to].pinned {
                self.nodes[syn.to].vx -= fx / self.nodes[syn.to].mass;
                self.nodes[syn.to].vy -= fy / self.nodes[syn.to].mass;
            }
        }

        for n in &mut self.nodes {
            if n.pinned {
                n.vx = 0.0;
                n.vy = 0.0;
                continue;
            }

            let dx = self.center_x - n.x;
            let dy = self.center_y - n.y;
            let dist_center = (dx * dx + dy * dy).sqrt();
            let grav = if dist_center > (self.width.min(self.height) * 0.3) {
                GRAVITY_STRONG
            } else {
                GRAVITY_WEAK
            };
            n.vx += dx * grav;
            n.vy += dy * grav;

            n.vx *= DAMPING;
            n.vy *= DAMPING;
            n.x += n.vx;
            n.y += n.vy;

            let margin = BOUNDS_MARGIN_MIN.max(n.radius);
            let x_min = margin;
            let x_max = (self.width - margin).max(x_min);
            let y_min = margin;
            let y_max = (self.height - margin).max(y_min);

            if n.x < x_min {
                n.vx += (x_min - n.x) * BOUNDS_REBOUND;
                n.x = x_min;
            }
            if n.x > x_max {
                n.vx -= (n.x - x_max) * BOUNDS_REBOUND;
                n.x = x_max;
            }
            if n.y < y_min {
                n.vy += (y_min - n.y) * BOUNDS_REBOUND;
                n.y = y_min;
            }
            if n.y > y_max {
                n.vy -= (n.y - y_max) * BOUNDS_REBOUND;
                n.y = y_max;
            }
        }
    }

    fn snapshot(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(self.nodes.len() * 4);
        for n in &self.nodes {
            out.push(n.x);
            out.push(n.y);
            out.push(n.vx);
            out.push(n.vy);
        }
        out
    }
}
