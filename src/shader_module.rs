pub struct ShaderModuleBuilder {
    modules: Vec<String>,
}

impl ShaderModuleBuilder {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    pub fn add_module(&mut self, code: &str) -> &mut Self {
        self.modules.push(code.to_string());
        return self;
    }

    pub fn build(&self, device: &wgpu::Device, label: Option<&str>) -> wgpu::ShaderModule {
        let combined = self.modules.join("\n\n");
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label,
            source: wgpu::ShaderSource::Wgsl(combined.into()),
        });
        return module;
    }
}
