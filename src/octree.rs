pub struct ChunkRequest {
    x: u32,
    y: u32,
    z: u32,
    lod: u32
}

pub struct Node {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub size: u32,
    pub lod: u32,
}

pub enum NodeResult {
    Empty,
    Projected { error: f32 }
}

pub struct Octree
{
    world_size: u32,
    chunk_size: u32,
    max_lod: u32,
    error_threshold: f32,
    test: fn(Node) -> NodeResult,
    emit: fn(&ChunkRequest),
}

impl Octree {
    pub fn from(
        world_size: u32,
        chunk_size: u32,
        max_lod: u32,
        error_threshold: f32,
        test: fn(Node) -> NodeResult, emit: fn(&ChunkRequest)
    ) -> Octree {
        Octree { world_size, chunk_size, max_lod, error_threshold, test, emit }
    }

    fn chunk(&self, x: u32, y: u32, z: u32, size: u32, lod: u32) {
        let step = self.chunk_size as usize;
        let lod_diff = (self.world_size / self.chunk_size).ilog2();
        // A node sufficient *above* the chunk boundary has lod < lod_diff; its
        // emitted chunks sit at the boundary, so the chunk-relative lod is 0.
        let chunk_lod = lod.saturating_sub(lod_diff);
        for x in (x..x + size).step_by(step) {
            for y in (y..y + size).step_by(step) {
                for z in (z..z + size).step_by(step) {
                    (self.emit)(&ChunkRequest { x, y, z, lod: chunk_lod })
                }
            }
        }
    }

    fn handle(&self, x: u32, y: u32, z: u32, size: u32, lod: u32) {
        match (self.test)(Node { x, y, z, size, lod })  {
            NodeResult::Empty => {
                log::debug!("Node at ({}, {}, {}) with size {} and LOD {} is empty", x, y, z, size, lod);
            },
            NodeResult::Projected { error } => {
                if error <= self.error_threshold {
                    log::debug!("Node at ({}, {}, {}) with size {} and LOD {} is projected with error {} <= {}, emitting chunk(s)", x, y, z, size, lod, error, self.error_threshold);
                    self.chunk(x, y, z, size, lod)
                } else {
                    if lod >= self.max_lod {
                        log::debug!("Node at ({}, {}, {}) with size {} is at max LOD {}, emitting chunk(s)", x, y, z, size, lod);
                        self.chunk(x, y, z, size, lod);
                    } else if size > self.chunk_size {
                        log::debug!("Node at ({}, {}, {}) with size {} and LOD {} is projected with error {}, descending", x, y, z, size, lod, error);
                        let new_size = size / 2;
                        let new_lod = lod + 1;
                        self.handle(x, y, z, new_size, new_lod);
                        self.handle(x, y, z + new_size, new_size, new_lod);
                        self.handle(x, y + new_size, z, new_size, new_lod);
                        self.handle(x, y + new_size, z + new_size, new_size, new_lod);
                        self.handle(x + new_size, y, z, new_size, new_lod);
                        self.handle(x + new_size, y, z + new_size, new_size, new_lod);
                        self.handle(x + new_size, y + new_size, z, new_size, new_lod);
                        self.handle(x + new_size, y + new_size, z + new_size, new_size, new_lod);
                    } else {
                        log::debug!("Node at ({}, {}, {}) with size {} and LOD {} is projected with error {}, but cannot descend further", x, y, z, size, lod, error);
                        self.handle(x, y, z, size, lod + 1);
                    }
                }
            }
        }       
    }

    pub fn iterate(&self) {
        self.handle(0, 0, 0, self.world_size, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    // `test` and `emit` are bare `fn` pointers and cannot capture state, so the
    // test harness routes them through thread-local storage. Each test owns its
    // own thread (Rust runs tests in parallel on separate threads), so the
    // thread-locals are effectively per-test scratch space.
    thread_local! {
        // Records every node passed to the test callback, in order.
        static TESTED: RefCell<Vec<Node>> = RefCell::new(Vec::new());
        // Records every emitted chunk, in order.
        static EMITTED: RefCell<Vec<ChunkRequest>> = RefCell::new(Vec::new());
        // The verdict the test callback should return for a given node. Indexed
        // by a key derived from (size, lod) so a test can script how the octree
        // sees the world at each level.
        static VERDICTS: RefCell<fn(&Node) -> NodeResult> = RefCell::new(|_| NodeResult::Empty);
    }

    impl Clone for Node {
        fn clone(&self) -> Self {
            Node { x: self.x, y: self.y, z: self.z, size: self.size, lod: self.lod }
        }
    }

    fn record_test(node: Node) -> NodeResult {
        let verdict = VERDICTS.with(|v| (*v.borrow())(&node));
        TESTED.with(|t| t.borrow_mut().push(node));
        verdict
    }

    fn record_emit(req: &ChunkRequest) {
        EMITTED.with(|e| e.borrow_mut().push(ChunkRequest {
            x: req.x, y: req.y, z: req.z, lod: req.lod,
        }));
    }

    /// Runs an octree with the given parameters and verdict function, returning
    /// (tested nodes, emitted chunks). `max_lod` defaults high so the LOD ceiling
    /// never interferes; tests that exercise the ceiling use `run_capped`.
    fn run(
        world_size: u32,
        chunk_size: u32,
        error_threshold: f32,
        verdicts: fn(&Node) -> NodeResult,
    ) -> (Vec<Node>, Vec<ChunkRequest>) {
        run_capped(world_size, chunk_size, u32::MAX, error_threshold, verdicts)
    }

    fn run_capped(
        world_size: u32,
        chunk_size: u32,
        max_lod: u32,
        error_threshold: f32,
        verdicts: fn(&Node) -> NodeResult,
    ) -> (Vec<Node>, Vec<ChunkRequest>) {
        TESTED.with(|t| t.borrow_mut().clear());
        EMITTED.with(|e| e.borrow_mut().clear());
        VERDICTS.with(|v| *v.borrow_mut() = verdicts);

        let octree = Octree::from(world_size, chunk_size, max_lod, error_threshold, record_test, record_emit);
        octree.iterate();

        let tested = TESTED.with(|t| t.borrow().clone());
        let emitted = EMITTED.with(|e| e.borrow().iter().map(|r| ChunkRequest {
            x: r.x, y: r.y, z: r.z, lod: r.lod,
        }).collect());
        (tested, emitted)
    }

    // Convention: `lod` is the subdivision count, starting at 0 for the root and
    // incrementing on each descent. Emitted ChunkRequests carry a *chunk-relative*
    // lod: node.lod minus the number of subdivisions needed to reach the chunk
    // boundary (log2(world_size / chunk_size)), so a node sufficient exactly at the
    // boundary emits lod 0.

    // A) An Empty node is never chunked and never descended into.
    #[test]
    fn empty_emits_nothing_and_does_not_descend() {
        let (tested, emitted) = run(8, 8, 1.0, |_| NodeResult::Empty);

        assert_eq!(emitted.len(), 0, "Empty must not emit chunks");
        // Only the single root node should have been tested; no descent.
        assert_eq!(tested.len(), 1, "Empty must not trigger descent");
        assert_eq!(tested[0].lod, 0, "root lod is 0");
        assert_eq!(tested[0].size, 8);
    }

    // B) Projected with error OVER threshold descends to a finer size (and LOD).
    #[test]
    fn projected_over_threshold_descends() {
        // world 16, chunk 8: root size 16 is > chunk_size, so an over-threshold
        // node descends to size 8. We make the root over-threshold and the
        // children empty so the recursion terminates after one descent.
        let (tested, emitted) = run(16, 8, 1.0, |node| {
            if node.size == 16 {
                NodeResult::Projected { error: 5.0 } // over threshold
            } else {
                NodeResult::Empty
            }
        });

        assert_eq!(emitted.len(), 0, "over-threshold root must not emit");
        // Root (size 16, lod 0) tested first, then the node re-scanned at size 8.
        let root = &tested[0];
        assert_eq!(root.size, 16);
        assert_eq!(root.lod, 0);

        let children: Vec<&Node> = tested.iter().filter(|n| n.size == 8).collect();
        assert!(!children.is_empty(), "must have descended to size 8");
        assert!(children.iter().all(|n| n.lod == 1), "descent increments LOD");
        // A 16-wide world at size 8 has 2*2*2 = 8 octants.
        assert_eq!(children.len(), 8);
    }

    // C) Projected with error UNDER (or equal to) threshold emits a chunk now.
    #[test]
    fn projected_under_threshold_emits_and_does_not_descend() {
        // world == chunk == 8: a single root node, sufficient immediately.
        // lod_diff = log2(8/8) = 0, so the chunk-relative lod equals the node lod (0).
        let (tested, emitted) = run(8, 8, 1.0, |_| NodeResult::Projected { error: 0.5 });

        assert_eq!(tested.len(), 1, "under-threshold must not descend");
        assert_eq!(emitted.len(), 1, "one root-sized chunk");
        assert_eq!(emitted[0].lod, 0, "world==chunk: chunk-relative lod is 0");
        assert_eq!((emitted[0].x, emitted[0].y, emitted[0].z), (0, 0, 0));
    }

    // D) A node that becomes sufficient BEFORE reaching the chunk boundary is
    //    split into multiple chunk_size-sized chunks.
    #[test]
    fn sufficient_above_chunk_boundary_yields_multiple_chunks() {
        // world 16, chunk 8, root size 16 (> chunk_size). Make the root
        // sufficient: it should emit (16/8)^3 = 8 chunks of size 8.
        let (tested, emitted) = run(16, 8, 1.0, |node| {
            if node.size == 16 {
                NodeResult::Projected { error: 0.5 } // under threshold immediately
            } else {
                NodeResult::Empty
            }
        });

        assert_eq!(tested.len(), 1, "root sufficient -> no descent");
        assert_eq!(emitted.len(), 8, "16-wide node splits into 8 chunks of size 8");

        // All eight chunk_size-aligned corners within the root must be present.
        let mut coords: Vec<(u32, u32, u32)> =
            emitted.iter().map(|r| (r.x, r.y, r.z)).collect();
        coords.sort();
        let mut expected: Vec<(u32, u32, u32)> = Vec::new();
        for x in [0, 8] {
            for y in [0, 8] {
                for z in [0, 8] {
                    expected.push((x, y, z));
                }
            }
        }
        expected.sort();
        assert_eq!(coords, expected);
        // Node lod 0 is *above* the chunk boundary (lod_diff 1); the chunk-relative
        // lod saturates to 0 rather than underflowing.
        assert!(emitted.iter().all(|r| r.lod == 0), "above-boundary chunks clamp to lod 0");
    }

    // E) A node that only becomes sufficient AT the chunk boundary yields a single
    //    chunk per octant, each at chunk-relative lod 0.
    #[test]
    fn sufficient_at_chunk_boundary_yields_single_chunk() {
        // world 16, chunk 8. Root (16, lod 0) is over-threshold -> descend to
        // size 8 (lod 1 == the chunk boundary). Sufficient there -> one chunk each.
        let (_, emitted) = run(16, 8, 1.0, |node| {
            if node.size == 16 {
                NodeResult::Projected { error: 5.0 } // descend
            } else {
                NodeResult::Projected { error: 0.5 } // sufficient at boundary
            }
        });

        // 8 octants at size 8, each emits exactly one chunk.
        assert_eq!(emitted.len(), 8);
        // Sufficiency reached exactly at the chunk boundary => chunk-relative lod 0.
        assert!(emitted.iter().all(|r| r.lod == 0), "boundary sufficiency => chunk lod 0");
        // Each emitted chunk is chunk_size-aligned and distinct.
        let mut coords: Vec<(u32, u32, u32)> =
            emitted.iter().map(|r| (r.x, r.y, r.z)).collect();
        coords.sort();
        coords.dedup();
        assert_eq!(coords.len(), 8, "no duplicate / oversized chunks");
    }

    // F) At the chunk boundary the over/under-threshold distinction is still
    //    respected: an over-threshold node at size == chunk_size descends in LOD
    //    only (size stays at chunk_size, never goes below it).
    #[test]
    fn over_threshold_at_chunk_boundary_descends_lod_not_size() {
        // world == chunk == 8: the root (lod 0) is already at the chunk boundary.
        // Over-threshold at lod 0, then sufficient at lod 1 to terminate.
        let (tested, emitted) = run(8, 8, 1.0, |node| {
            if node.lod == 0 {
                NodeResult::Projected { error: 5.0 } // over threshold at boundary
            } else {
                NodeResult::Projected { error: 0.5 } // sufficient at next LOD
            }
        });

        // Size never drops below chunk_size.
        assert!(tested.iter().all(|n| n.size == 8), "size must not descend past chunk_size");
        // Both the lod-0 (over) and lod-1 (under) nodes were tested.
        assert!(tested.iter().any(|n| n.lod == 0));
        assert!(tested.iter().any(|n| n.lod == 1), "must descend in LOD at the boundary");
        // The sufficient lod-1 node emits exactly one chunk, one subdivision into
        // the chunk boundary => chunk-relative lod 1.
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0].lod, 1);
    }

    // G) max_lod caps the LOD-only descent: at the ceiling an over-threshold node
    //    is emitted as a best-effort chunk instead of recursing forever.
    #[test]
    fn max_lod_caps_lod_descent_and_emits() {
        // world == chunk == 8 (always at the boundary), max_lod 2, and *every*
        // node is over-threshold. Without the cap this would recurse on lod
        // forever; with it, lod 2 emits.
        let (tested, emitted) = run_capped(8, 8, 2, 1.0, |_| NodeResult::Projected { error: 5.0 });

        // Tested at lod 0, 1, 2 then stops.
        assert_eq!(tested.iter().map(|n| n.lod).max(), Some(2), "must not descend past max_lod");
        assert!(tested.iter().all(|n| n.size == 8));
        // Exactly one chunk emitted, at the cap. lod_diff = 0, so chunk lod == 2.
        assert_eq!(emitted.len(), 1, "cap emits a best-effort chunk");
        assert_eq!(emitted[0].lod, 2);
    }

    // H) Descent is bounded to the parent octant: a single non-origin subregion
    //    that becomes sufficient emits a chunk only at its own corner, never
    //    leaking toward the world origin or the opposite corner.
    #[test]
    fn descent_stays_within_parent_octant() {
        // world 16, chunk 8. Root (size 16, lod 0) is over-threshold and
        // subdivides into 8 octants of size 8. Only the (8,8,8) octant is
        // sufficient; the rest are empty.
        let (_, emitted) = run(16, 8, 1.0, |node| {
            if node.size == 16 {
                NodeResult::Projected { error: 5.0 } // root over -> subdivide
            } else if (node.x, node.y, node.z) == (8, 8, 8) {
                NodeResult::Projected { error: 0.5 } // this octant sufficient
            } else {
                NodeResult::Empty
            }
        });

        // Exactly one chunk, at the sufficient octant's corner — not (0,0,0).
        assert_eq!(emitted.len(), 1, "only the sufficient octant emits");
        assert_eq!((emitted[0].x, emitted[0].y, emitted[0].z), (8, 8, 8),
            "chunk must land in its own octant, not leak toward the origin");
    }

    // I) Tiling invariant on an adaptive, mixed-depth tree.
    //
    // Given the upstream guarantee that chunk_size divides world_size (a power of
    // two apart), the emitted chunks must *exactly tile* the occupied space at
    // chunk_size granularity: every occupied unit cell covered exactly once, no
    // empty cell ever covered, and no off-unit / out-of-bounds request.
    //
    // The check works by consuming a "universe" of occupied unit cells: each
    // emitted ChunkRequest expands into the unit cells it claims and removes each
    // one. Removing a cell that isn't present (off-grid, in empty space, or
    // already covered) is a panic; any cell left over at the end is a gap.
    #[test]
    fn adaptive_tree_tiles_occupied_space_exactly() {
        use std::collections::HashSet;

        // Ground-truth occupancy at unit-cell (chunk_size) granularity. World 16,
        // chunk 4 => a 4x4x4 grid of unit cells indexed in [0,4)^3.
        //
        // Occupancy is defined per *size-8 octant* (each spans a 2x2x2 unit block)
        // so the same scene exercises: a fully-empty octant, fully-full octants
        // that should stay coarse, and a partly-full octant that must descend and
        // leave its empty children uncovered.
        fn occupied(ux: u32, uy: u32, uz: u32) -> bool {
            // Which size-8 octant (corner in {0,8}) does this unit cell fall in?
            let oct = (ux / 2, uy / 2, uz / 2); // each component 0 or 1
            match oct {
                // Fully empty octant: emits nothing.
                (0, 0, 0) => false,
                // Partly-full octant: only the unit cells with even ux are full,
                // forcing descent to size 4 with some empty size-4 children.
                (0, 1, 0) => ux % 2 == 0,
                // Every other octant is fully occupied (stays coarse at size 8).
                _ => true,
            }
        }

        // Build the universe of occupied unit cells.
        let mut universe: HashSet<(u32, u32, u32)> = HashSet::new();
        for ux in 0..4 {
            for uy in 0..4 {
                for uz in 0..4 {
                    if occupied(ux, uy, uz) {
                        universe.insert((ux, uy, uz));
                    }
                }
            }
        }
        assert!(!universe.is_empty(), "scene must have occupied cells");
        assert!(universe.len() < 64, "scene must have some empty cells");

        // A node is Empty iff *no* unit cell under it is occupied. Otherwise it is
        // sufficient (low error) exactly when it is fully occupied; a partly-full
        // node is over-threshold so it must descend. This derives the verdict from
        // the same ground truth, so verdict and universe cannot drift apart.
        fn verdict(node: &Node) -> NodeResult {
            let units = node.size / 4; // unit cells per side under this node
            let (bx, by, bz) = (node.x / 4, node.y / 4, node.z / 4);
            let mut any = false;
            let mut all = true;
            for dx in 0..units {
                for dy in 0..units {
                    for dz in 0..units {
                        if occupied(bx + dx, by + dy, bz + dz) {
                            any = true;
                        } else {
                            all = false;
                        }
                    }
                }
            }
            if !any {
                NodeResult::Empty
            } else if all {
                NodeResult::Projected { error: 0.0 } // fully occupied -> sufficient
            } else {
                NodeResult::Projected { error: 1.0 } // mixed -> must descend
            }
        }

        let (tested, emitted) = run(16, 4, 0.5, verdict);

        assert!(!emitted.is_empty(), "occupied space must emit chunks");

        // Guard against a vacuous pass: the scene must genuinely be adaptive.
        //  - A fully-occupied size-8 octant is sufficient (emitted) at size 8.
        //  - The partly-full octant must descend and test nodes at size 4.
        assert!(
            tested.iter().any(|n| n.size == 8 && matches!(verdict(n), NodeResult::Projected { error } if error <= 0.5)),
            "expected at least one coarse (size-8) octant that is sufficient"
        );
        assert!(
            tested.iter().any(|n| n.size == 4),
            "expected at least one branch to descend to the fine (size-4) level"
        );

        // Consume the universe. In this scene every emitted chunk is exactly
        // chunk_size, so each ChunkRequest maps to a single unit cell.
        let mut remaining = universe.clone();
        for c in &emitted {
            // Each ChunkRequest is one chunk_size-aligned cell.
            let cell = (c.x / 4, c.y / 4, c.z / 4);
            assert_eq!((c.x % 4, c.y % 4, c.z % 4), (0, 0, 0),
                "chunk origin must be chunk_size-aligned: {:?}", (c.x, c.y, c.z));
            assert!(cell.0 < 4 && cell.1 < 4 && cell.2 < 4,
                "chunk out of world bounds: {:?}", (c.x, c.y, c.z));
            assert!(remaining.remove(&cell),
                "chunk at {:?} covers an empty or already-covered cell (overlap / spill into empty space)",
                (c.x, c.y, c.z));
        }

        assert!(remaining.is_empty(),
            "occupied cells left uncovered (gap in tiling): {:?}", remaining);
    }
}