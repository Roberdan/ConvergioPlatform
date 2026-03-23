use std::collections::HashMap;

pub struct SpatialHash {
    cell_size: f32,
    inv_cell: f32,
    grid: HashMap<(i32, i32), Vec<usize>>,
}

impl SpatialHash {
    pub fn new(cell_size: f32) -> Self {
        let safe_cell = cell_size.max(1.0);
        Self {
            cell_size: safe_cell,
            inv_cell: 1.0 / safe_cell,
            grid: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.grid.clear();
    }

    pub fn set_cell_size(&mut self, cell_size: f32) {
        let safe_cell = cell_size.max(1.0);
        if (safe_cell - self.cell_size).abs() > f32::EPSILON {
            self.cell_size = safe_cell;
            self.inv_cell = 1.0 / safe_cell;
            self.grid.clear();
        }
    }

    pub fn insert(&mut self, idx: usize, x: f32, y: f32) {
        let cx = (x * self.inv_cell).floor() as i32;
        let cy = (y * self.inv_cell).floor() as i32;
        self.grid.entry((cx, cy)).or_default().push(idx);
    }

    pub fn get_nearby(&self, x: f32, y: f32, radius: f32) -> Vec<usize> {
        let mut nearby = Vec::new();
        let min_x = ((x - radius) * self.inv_cell).floor() as i32;
        let max_x = ((x + radius) * self.inv_cell).floor() as i32;
        let min_y = ((y - radius) * self.inv_cell).floor() as i32;
        let max_y = ((y + radius) * self.inv_cell).floor() as i32;

        for cx in min_x..=max_x {
            for cy in min_y..=max_y {
                if let Some(bucket) = self.grid.get(&(cx, cy)) {
                    nearby.extend(bucket.iter().copied());
                }
            }
        }

        nearby
    }
}
