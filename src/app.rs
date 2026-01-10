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

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            },
            WindowEvent::RedrawRequested => {
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.

                // Draw.

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                //self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }
}