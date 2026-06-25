use serde::Deserialize;
use serde_yaml;
use log;

#[derive(Deserialize, Debug)]
pub struct SceneSpec {
    world_size: u32
}

pub struct Scene {
    
}

impl SceneSpec {
    pub fn parse(path: &String) -> Result<SceneSpec, String> {
        let file = std::fs::File::open(path).map_err(|e|e.to_string())?;
        let spec: SceneSpec = serde_yaml::from_reader(&file).map_err(|e|e.to_string())?;
        log::debug!("loaded spec {:?}", spec);
        Ok(spec)
    }

    pub fn build(&self) -> Scene {
        Scene {}
    }
}