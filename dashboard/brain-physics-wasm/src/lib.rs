mod spatial;
mod step;

use spatial::SpatialHash;
use wasm_bindgen::prelude::*;

pub(crate) const REPULSION_K: f32 = 90.0;
pub(crate) const REPULSION_RADIUS_MULTIPLIER: f32 = 3.5;
pub(crate) const REPULSION_FORCE_SCALE: f32 = 0.5;
pub(crate) const SPRING_FORCE: f32 = 0.002;
pub(crate) const SPRING_SESSION_MULTIPLIER: f32 = 2.2;
pub(crate) const SPRING_DEFAULT_MULTIPLIER: f32 = 1.4;
pub(crate) const GRAVITY_STRONG: f32 = 0.01;
pub(crate) const GRAVITY_WEAK: f32 = 0.0015;
pub(crate) const DAMPING: f32 = 0.82;
pub(crate) const BOUNDS_MARGIN_MIN: f32 = 20.0;
pub(crate) const BOUNDS_REBOUND: f32 = 0.3;
pub(crate) const GRID_SIZE: f32 = 50.0;
pub(crate) const MAX_REPULSION_FORCE: f32 = 3.0;

#[wasm_bindgen]
pub struct PhysicsEngine {
    pub(crate) nodes: Vec<Node>,
    pub(crate) synapses: Vec<Synapse>,
    pub(crate) spatial: SpatialHash,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) center_x: f32,
    pub(crate) center_y: f32,
}

#[derive(Clone)]
pub(crate) struct Node {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) vx: f32,
    pub(crate) vy: f32,
    pub(crate) radius: f32,
    pub(crate) mass: f32,
    pub(crate) pinned: bool,
    pub(crate) is_session: bool,
}

pub(crate) struct Synapse {
    pub(crate) from: usize,
    pub(crate) to: usize,
    pub(crate) strength: f32,
    pub(crate) spring_length: f32,
    pub(crate) is_session_link: bool,
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
