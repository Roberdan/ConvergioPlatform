use crate::{
    Node, PhysicsEngine, Synapse, BOUNDS_MARGIN_MIN, BOUNDS_REBOUND, DAMPING, GRAVITY_STRONG,
    GRAVITY_WEAK, GRID_SIZE, MAX_REPULSION_FORCE, REPULSION_FORCE_SCALE, REPULSION_K,
    REPULSION_RADIUS_MULTIPLIER, SPRING_DEFAULT_MULTIPLIER, SPRING_FORCE, SPRING_SESSION_MULTIPLIER,
};

impl PhysicsEngine {
    pub(crate) fn single_step(&mut self) {
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

        apply_spring_forces(&mut self.nodes, &self.synapses, node_count);
        apply_gravity_and_bounds(
            &mut self.nodes,
            self.center_x,
            self.center_y,
            self.width,
            self.height,
        );
    }

    pub(crate) fn snapshot(&self) -> Vec<f32> {
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

fn apply_spring_forces(nodes: &mut Vec<Node>, synapses: &[Synapse], node_count: usize) {
    for syn in synapses {
        if syn.from >= node_count || syn.to >= node_count || syn.from == syn.to {
            continue;
        }

        let (ax, ay) = (nodes[syn.from].x, nodes[syn.from].y);
        let (bx, by) = (nodes[syn.to].x, nodes[syn.to].y);
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

        if !nodes[syn.from].pinned {
            nodes[syn.from].vx += fx / nodes[syn.from].mass;
            nodes[syn.from].vy += fy / nodes[syn.from].mass;
        }
        if !nodes[syn.to].pinned {
            nodes[syn.to].vx -= fx / nodes[syn.to].mass;
            nodes[syn.to].vy -= fy / nodes[syn.to].mass;
        }
    }
}

fn apply_gravity_and_bounds(
    nodes: &mut Vec<Node>,
    center_x: f32,
    center_y: f32,
    width: f32,
    height: f32,
) {
    for n in nodes {
        if n.pinned {
            n.vx = 0.0;
            n.vy = 0.0;
            continue;
        }

        let dx = center_x - n.x;
        let dy = center_y - n.y;
        let dist_center = (dx * dx + dy * dy).sqrt();
        let grav = if dist_center > (width.min(height) * 0.3) {
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
        let x_max = (width - margin).max(x_min);
        let y_min = margin;
        let y_max = (height - margin).max(y_min);

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
