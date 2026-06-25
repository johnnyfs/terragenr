use sdl3;
use sdl3::Sdl;
use sdl3::video::Window;
use sdl3::gpu::{Device, ShaderFormat};
use sdl3_sys;
use log;

use crate::args::Args;

pub struct Video {
    pub sdl: Sdl,
    pub window: Window,
    pub device: Device
}

impl Video {
    pub fn from(args: &Args) -> Result<Video, String> {
        let sdl = sdl3::init().map_err(|e|e.to_string())?;
        let video_subsystem = sdl.video().map_err(|e|e.to_string())?;

        let window = video_subsystem.window("terragen", args.width, args.height)
            .position_centered()
            .build()
            .map_err(|e|e.to_string())?;

        let device = Device::new(ShaderFormat::SPIRV | ShaderFormat::METALLIB, args.debug_gpu)
            .map_err(|e|e.to_string())?;

        unsafe {
            let name = sdl3_sys::gpu::SDL_GetGPUDeviceDriver(device.raw());
            let name_str = std::ffi::CStr::from_ptr(name).to_string_lossy();
            log::info!("Using device '{}' (debug: {})", name_str, args.debug_gpu);
        }

        let device = device.with_window(&window).map_err(|e|e.to_string())?;

        Ok(Video { sdl, window, device })
    }
}