use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowId};

use std::sync::Arc;

use crate::renderer::Renderer;

pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
}

impl Default for App {
    fn default() -> Self {
        Self { window: None,
               renderer: None, }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);

        let window = Arc::new(
            event_loop.create_window(Window::default_attributes()).unwrap()
        );

        self.window = Some(window.clone());
        self.renderer = Some(Renderer::new(window));

    }

    fn window_event(&mut self, 
                    event_loop: &ActiveEventLoop, 
                    id: WindowId, 
                    event: WindowEvent) {

        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            },
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.render().unwrap();
                }
            }
            _ => (),
        }

    }
}