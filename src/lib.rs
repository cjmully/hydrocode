pub mod mls_mpm;
pub mod shader_module;
pub mod texture;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
    cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);

// struct State<'a> {
//     surface: wgpu::Surface<'a>,
//     device: wgpu::Device,
//     queue: wgpu::Queue,
//     config: wgpu::SurfaceConfiguration,
//     size: winit::dpi::PhysicalSize<u32>,
//     window: &'a Window,
// }

// impl<'a> State<'a> {
//     // Creating some of the wgpu types requires async code
//     async fn new(window: &'a Window) -> State<'a> {
//         todo!()
//     }

//     pub fn window(&self) -> &Window {
//         &self.window
//     }

//     fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
//         todo!()
//     }

//     fn input(&mut self, event: &WindowEvent) -> bool {
//         todo!()
//     }

//     fn update(&mut self) {
//         todo!()
//     }

//     fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
//         todo!()
//     }
// }
