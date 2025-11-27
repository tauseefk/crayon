pub struct CRRenderPipeline {
    _layout: wgpu::PipelineLayout,
    pub pipeline: wgpu::RenderPipeline,
}

impl CRRenderPipeline {
    pub fn new(
        device: &wgpu::Device,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
        shader: wgpu::ShaderModule,
        format: wgpu::TextureFormat,
        buffers: &[wgpu::VertexBufferLayout<'static>],
        blend: bool,
        label: &str,
    ) -> anyhow::Result<Self> {
        let layout = CRRenderPipeline::create_pipeline_layout(device, bind_group_layouts, label)?;
        let pipeline = CRRenderPipeline::create_pipeline(
            device, &layout, shader, format, buffers, blend, label,
        )?;

        Ok(Self {
            _layout: layout,
            pipeline,
        })
    }

    fn create_pipeline_layout(
        device: &wgpu::Device,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
        label: &str,
    ) -> anyhow::Result<wgpu::PipelineLayout> {
        Ok(
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(&format!("{} Layout", label)),
                bind_group_layouts,
                push_constant_ranges: &[],
            }),
        )
    }

    fn create_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        shader: wgpu::ShaderModule,
        format: wgpu::TextureFormat,
        buffers: &[wgpu::VertexBufferLayout<'static>],
        blend: bool,
        label: &str,
    ) -> anyhow::Result<wgpu::RenderPipeline> {
        let blend = match blend {
            true => Some(wgpu::BlendState::ALPHA_BLENDING),
            false => Some(wgpu::BlendState::REPLACE),
        };

        Ok(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            }),
        )
    }
}
