use hydrocode::*;
use renderer::Renderer;
use winit::event_loop::EventLoop;

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut renderer = Renderer::default();
    event_loop.run_app(&mut renderer).unwrap();
}
