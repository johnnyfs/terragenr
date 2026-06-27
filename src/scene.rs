use std::{collections::HashMap, ops::Range};

use ego_tree::Tree;
use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use serde_yaml;
use log;
use validator::Validate;
use glam::Vec3;
use crate::octree::{Octree, NodeResult};

#[derive(Deserialize, Debug)]
pub struct SdfSpec {
    height_range: Range<u32>
}

#[derive(Deserialize, Debug)]
pub enum SdfParameterOverride {
    HeightRange { height_range: Range<u32> }
}

#[derive(Deserialize, Debug)]
pub enum SceneNodeType  {
    Plane { pos: Vec3, size: Vec3 },
}

#[derive(Deserialize, Debug, Validate)]
pub struct SceneNodeSpec {
    kind: SceneNodeType,
    name: String,
    overrides: Vec<SdfParameterOverride>,
}

#[serde_inline_default]
#[derive(Deserialize, Debug, Validate)]
pub struct SceneSpec {
    #[validate(range(min = 1))]
    world_size: u32,

    #[validate(range(min = 1))]
    chunk_size: u32,

    #[serde_inline_default(255)]
    max_lod: u32,

    sdf: SdfSpec,

    nodes: Tree<SceneNodeSpec>
}

pub struct Scene {
    octree: Octree
}

impl SceneSpec {
    pub fn parse(path: &String) -> Result<SceneSpec, String> {
        let file = std::fs::File::open(path).map_err(|e|e.to_string())?;
        let spec: SceneSpec = serde_yaml::from_reader(&file).map_err(|e|e.to_string())?;
        log::debug!("loaded spec {:?}", spec);
        spec.validate().map_err(|e| e.to_string())?;
        if spec.world_size % spec.chunk_size != 0 {
            return Err(format!("world_size {} must be a multiple of chunk_size {}", spec.world_size, spec.chunk_size));
        }
        Ok(spec)
    }

    pub fn build(&self) -> Scene {
        let octree = Octree::from(
            self.world_size,
            self.chunk_size,
            self.max_lod,
            0.1,
            |node|{
                if node.size == 32 {
                    NodeResult::Projected{ error: 0.0 }
                } else {
                    NodeResult::Projected{ error: 1.0 }
                }
            },
            |_|{}
        );
        Scene { octree }
    }
}

impl Scene {
    pub fn render(&self, video: &crate::video::Video) {
        self.octree.iterate();
        log::debug!("rendering scene");
    }
}