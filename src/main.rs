mod app;

use app::App;

use std::{error::Error, sync::Arc};
use winit::event_loop::EventLoop;



fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(&mut App::default()).unwrap();
}
