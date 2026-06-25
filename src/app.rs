use sdl3::event::Event;

use crate::args;
use crate::scene::{Scene, SceneSpec};
use crate::video::{Video};

pub struct App {
    scene: Scene,
    video: Video
}

impl App {
    pub fn from(args: &args::Args) -> Result<App, String> {
        let video = Video::from(&args).map_err(|e|e.to_string())?;

        let spec: SceneSpec = SceneSpec::parse(&args.scene_spec_path)?;
        let scene = spec.build();

        Ok(App { scene, video })
    }

    pub fn run(&self) -> Result<(), String> {
        let mut events = self.video.sdl.event_pump().map_err(|e|e.to_string())?;

        'game: loop { 
            self.scene.render(&self.video);

            for event in events.poll_iter() {
                match event {
                    Event::Quit { .. } => break 'game,
                    _ => { }
                }
            }
        }
        
        Ok(())
    }
}