pub struct ChunkRequest {
    x: u32,
    y: u32,
    lod: u8
}

pub struct Node {
    pub x: u32,
    pub y: u32,
    pub size: u32,
    pub lod: u8
}

pub enum NodeResult {
    Empty,
    Projected { error: f32 }
}

pub struct Octree
{
    world_size: u32,
    chunk_size: u32,
    error_threshold: f32,
    test: fn(Node) -> NodeResult,
    emit: fn(&ChunkRequest),
}

impl Octree {
    pub fn from(world_size: u32, chunk_size: u32, error_threshold: f32, test: fn(Node) -> NodeResult, emit: fn(&ChunkRequest) ) -> Octree {
        Octree { world_size, chunk_size, error_threshold, test, emit }
    }

    fn chunk(&self, x: u32, y: u32, size: u32, lod: u8) {
        let step = size.try_into().unwrap();
        for x in (x..x + size).step_by(step) {
            for y in (y..y + size).step_by(step) {
                (self.emit)(&ChunkRequest { x, y, lod })
            }
        }
    }

    fn descend(&self, size: u32, lod: u8) {
        let step = size.try_into().unwrap();
        for x in (0..self.world_size).step_by(step) {
            for y in (0..self.world_size).step_by(step) {
                match (self.test)(Node { x, y, size, lod })  {
                    NodeResult::Empty => {
                        log::debug!("Node at ({}, {}) with size {} and LOD {} is empty", x, y, size, lod);
                    },
                    NodeResult::Projected { error } => {
                        if error <= self.error_threshold {
                            log::debug!("Node at ({}, {}) with size {} and LOD {} is projected with error {} <= {}, emitting chunk(s)", x, y, size, lod, error, self.error_threshold);
                            self.chunk(x, y, size, lod)
                        } else {
                            if size > self.chunk_size {
                                log::debug!("Node at ({}, {}) with size {} and LOD {} is projected with error {}, descending", x, y, size, lod, error);
                                self.descend(size / 2, lod + 1)
                            } else {
                                log::debug!("Node at ({}, {}) with size {} and LOD {} is projected with error {}, but cannot descend further", x, y, size, lod, error);
                                self.descend(size, lod + 1)
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn iterate(&self) {
        self.descend(self.world_size, 1)
    }
}