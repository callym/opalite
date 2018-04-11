use std::{ mem, sync::{ Arc, Mutex } };
use cgmath::Vector3;
use conrod::{ self, render::{ self, PrimitiveWalker }, text::{ rt, GlyphCache } };
use rusttype;
use failure::Error;
use crate::{ Config, OpalUi, Resources, RLock };
use crate::renderer::{ self, Buffer, BufferData, Image, Model, RenderError, PushConstant, Sampler, ShaderKey, Shader };
use crate::renderer::pipe::{ PipeKey, Pipe };
use crate::renderer::model::{ ModelData, UiVertex };

use back;
use back::Backend as B;

use hal;
use hal::{ command, format as f, image as i, pass, pso };
use hal::{ Backend, Device };
use hal::{
    DescriptorPool,
    Primitive,
    Backbuffer,
};
use hal::format::Swizzle;
use hal::pass::Subpass;
use hal::pso::{ PipelineStage, ShaderStageFlags };

#[derive(PushConstant, Serialize, Copy, Clone, Debug)]
pub struct ModelLocals {
    pub model: [[f32; 4]; 4],
}

#[derive(BufferData, Serialize, Copy, Clone, Debug)]
#[uniform]
pub struct Locals {
    pub proj_view: [[f32; 4]; 4],
}

enum Mode {
    Text = 0,
    Image = 1,
    Geometry = 2,
}

pub struct UiPipe<'a> {
    dimensions: (u32, u32),
    device: Arc<Mutex<back::Device>>,
    dpi_factor: f32,
    viewport: pso::Viewport,
    pipeline_layout: <B as Backend>::PipelineLayout,
    render_pass: <B as Backend>::RenderPass,
    pipeline: <B as Backend>::GraphicsPipeline,
    desc_set: <B as Backend>::DescriptorSet,
    framebuffers: Vec<<B as Backend>::Framebuffer>,
    locals: Buffer<Locals, B>,
    sampler: Arc<Sampler<B>>,
    glyph_cache: GlyphCache<'a>,
    cache_tex: Image<B>,
}

impl<'a> Pipe for UiPipe<'a> {
    type Locals = Locals;
    type Models = Model<UiVertex>;
    type ModelsLocals = ModelLocals;

    fn key(&self) -> PipeKey {
        PipeKey(String::from("Ui"))
    }

    fn locals(&self) -> &Buffer<Self::Locals, B> {
        &self.locals
    }

    fn locals_mut(&mut self) -> &mut Buffer<Self::Locals, B> {
        &mut self.locals
    }
}

impl<'a> UiPipe<'a> {
    pub fn draw(&mut self, command_buffer: &mut command::CommandBuffer<B, hal::Graphics>, memory_types: &[hal::MemoryType], frame_id: usize, opal_ui: &mut OpalUi) {
        let Self {
            device,
            dimensions,
            dpi_factor,
            pipeline_layout,
            render_pass,
            pipeline,
            desc_set,
            framebuffers,
            viewport,
            glyph_cache,
            cache_tex,
            ..
        } = self;

        if cache_tex.submitted == false {
            cache_tex.submit(command_buffer);
        }

        let (width, height) = (dimensions.0 as f32, dimensions.1 as f32);

        let ratio = width / height;

        let mut index = 0;
        let mut vertices = vec![];
        let mut indices = vec![];
        let mut ui = vec![];

        #[derive(Copy, Clone, Debug, PartialEq, Eq)]
        enum UiState { None, Plain, Image, Text };

        let mut current_state = UiState::None;

        let vx = |x: f64| {
            if ratio > 1.0 {
                (x as f32) * (dimensions.1 as f32 / dimensions.0 as f32)
            } else {
                x as f32
            }
        };

        let vy = |y: f64| {
            if ratio < 1.0 {
                (y as f32) * (dimensions.0 as f32 / dimensions.1 as f32)
            } else {
                y as f32
            }
        };

        let finish_state = |vertices: &mut Vec<UiVertex>, indices: &mut Vec<u32>, index: &mut u32| {
            if vertices.is_empty() || indices.is_empty() {
                return None;
            }

            let mut vertex_buffer = Buffer::<UiVertex, B>::new(device.clone(), vertices.len() as u64, hal::buffer::Usage::VERTEX, &memory_types).unwrap();
            vertex_buffer.write(&vertices[..]).unwrap();

            let mut index_buffer = Buffer::<u32, B>::new(device.clone(), indices.len() as u64, hal::buffer::Usage::INDEX, &memory_types).unwrap();
            index_buffer.write(&indices[..]).unwrap();

            vertices.clear();
            indices.clear();
            *index = 0;

            Some(Model { vertex_buffer, index_buffer })
        };

        if opal_ui.is_some() {
            let opal_ui = opal_ui.as_mut().unwrap();
            let mut opal_ui = opal_ui.walk();

            ui.clear();

            while let Some(primitive) = opal_ui.next_primitive() {
                let render::Primitive { kind, rect, .. } = primitive;

                match kind {
                    render::PrimitiveKind::Rectangle { color } => {
                        if current_state != UiState::Plain {
                            finish_state(&mut vertices, &mut indices, &mut index)
                                .map(|m| ui.push(m));
                            current_state = UiState::Plain;
                        }

                        let (l, r, b, t) = rect.l_r_b_t();
                        let v = |x, y| {
                            UiVertex {
                                position: [vx(x), vy(y)].into(),
                                color: color.to_fsa().into(),
                                uv: [0.0, 0.0].into(),
                                mode: Mode::Geometry as u32,
                            }
                        };

                        // Bottom left triangle.
                        vertices.push(v(l, t));
                        vertices.push(v(r, b));
                        vertices.push(v(l, b));

                        // Top right triangle.
                        vertices.push(v(l, t));
                        vertices.push(v(r, b));
                        vertices.push(v(r, t));

                        indices.push(index);
                        indices.push(index + 1);
                        indices.push(index + 2);
                        indices.push(index + 3);
                        indices.push(index + 4);
                        indices.push(index + 5);
                        index += 6;
                    },
                    render::PrimitiveKind::TrianglesSingleColor { color, triangles } => {
                        if triangles.is_empty() {
                            continue;
                        }

                        if current_state != UiState::Plain {
                            finish_state(&mut vertices, &mut indices, &mut index)
                                .map(|m| ui.push(m));
                            current_state = UiState::Plain;
                        }

                        let v = |p: [f64; 2]| {
                            let color: [f32; 4] = color.into();
                            UiVertex {
                                position: [vx(p[0]), vy(p[1])].into(),
                                color: color.into(),
                                uv: [0.0, 0.0].into(),
                                mode: Mode::Geometry as u32,
                            }
                        };

                        for triangle in triangles {
                            vertices.push(v(triangle[0]));
                            vertices.push(v(triangle[1]));
                            vertices.push(v(triangle[2]));

                            indices.push(index);
                            indices.push(index + 1);
                            indices.push(index + 2);
                            index += 3;
                        }
                    }
                    render::PrimitiveKind::TrianglesMultiColor { triangles } => {
                        if triangles.is_empty() {
                            continue;
                        }

                        if current_state != UiState::Plain {
                            finish_state(&mut vertices, &mut indices, &mut index)
                                .map(|m| ui.push(m));
                            current_state = UiState::Plain;
                        }

                        let v = |(p, c): ([f64; 2], conrod::color::Rgba)| {
                            let c: [f32; 4] = c.into();
                            UiVertex {
                                position: [vx(p[0]), vy(p[1])].into(),
                                color: c.into(),
                                uv: [0.0, 0.0].into(),
                                mode: Mode::Geometry as u32,
                            }
                        };

                        for triangle in triangles {
                            vertices.push(v(triangle[0]));
                            vertices.push(v(triangle[1]));
                            vertices.push(v(triangle[2]));

                            indices.push(index);
                            indices.push(index + 1);
                            indices.push(index + 2);
                            index += 3;
                        }
                    }
                    render::PrimitiveKind::Text { color, text, font_id } => {
                        if current_state != UiState::Text {
                            finish_state(&mut vertices, &mut indices, &mut index)
                                .map(|m| ui.push(m));
                            current_state = UiState::Text;
                        }

                        let positioned_glyphs = text.positioned_glyphs(*dpi_factor);

                        for glyph in positioned_glyphs {
                            glyph_cache.queue_glyph(font_id.index(), glyph.clone());
                        }

                        glyph_cache.cache_queued(|rect, data| {
                            let offset = [rect.min.x as usize, rect.min.y as usize];
                            let size = [rect.width() as usize, rect.height() as usize];

                            let new_data = data.iter().map(|x| [255, 255, 255, *x]).collect::<Vec<_>>();

                            cache_tex.update(offset, size, &new_data[..], device.clone()).unwrap();
                        }).unwrap();

                        let color = color.to_fsa();
                        let cache_id = font_id.index();
                        let origin = rt::point(0.0, 0.0);

                        let to_gl_rect = |screen_rect: rt::Rect<i32>| rt::Rect {
                            min: origin + (rt::vector(
                                    screen_rect.min.x as f32 / width - 0.5,
                                    screen_rect.min.y as f32 / height - 0.5)
                                ) * 2.0,
                            max: origin + (rt::vector(
                                    screen_rect.max.x as f32 / width - 0.5,
                                    screen_rect.max.y as f32 / height - 0.5)
                                ) * 2.0,
                        };

                        for g in positioned_glyphs {
                            if let Ok(Some((uv_rect, screen_rect))) = glyph_cache.rect_for(cache_id, g) {
                                let gl_rect = to_gl_rect(screen_rect);
                                let color: [f32; 4] = color.into();
                                let v = |p: [f32; 2], t: [f32; 2]| UiVertex {
                                    position: p.into(),
                                    color: color.into(),
                                    uv: t.into(),
                                    mode: Mode::Text as u32,
                                };

                                vertices.push(v([gl_rect.min.x, gl_rect.max.y], [uv_rect.min.x, uv_rect.max.y]));
                                vertices.push(v([gl_rect.min.x, gl_rect.min.y], [uv_rect.min.x, uv_rect.min.y]));
                                vertices.push(v([gl_rect.max.x, gl_rect.min.y], [uv_rect.max.x, uv_rect.min.y]));
                                vertices.push(v([gl_rect.max.x, gl_rect.min.y], [uv_rect.max.x, uv_rect.min.y]));
                                vertices.push(v([gl_rect.max.x, gl_rect.max.y], [uv_rect.max.x, uv_rect.max.y]));
                                vertices.push(v([gl_rect.min.x, gl_rect.max.y], [uv_rect.min.x, uv_rect.max.y]));

                                indices.push(index);
                                indices.push(index + 1);
                                indices.push(index + 2);
                                indices.push(index + 3);
                                indices.push(index + 4);
                                indices.push(index + 5);
                                index += 6;
                            }
                        }
                    }
                    _ => {
                        println!("Unsupported");
                        println!("index: {}", indices.len());
                        current_state = UiState::None;
                        index = 0;
                    },
                }
            }
        }

        if current_state != UiState::None {
            finish_state(&mut vertices, &mut indices, &mut index)
                .map(|m| ui.push(m));
        }

        cache_tex.submit(command_buffer);

        if ui.is_empty() == false {
            command_buffer.set_viewports(&[viewport.clone()]);
            command_buffer.set_scissors(&[viewport.rect]);
            command_buffer.bind_graphics_pipeline(pipeline);

            let mut encoder = command_buffer.begin_render_pass_inline(
                &render_pass,
                &framebuffers[frame_id],
                viewport.rect,
                &[],
            );
            encoder.bind_graphics_descriptor_sets(pipeline_layout, 0, Some(desc_set)); //TODO

            for model in ui {
                let locals = {
                    let model_data: ModelData = Default::default();
                    let position = Vector3::new(0, 0, 0);
                    let model_data = model_data.to_matrix(&position);

                    ModelLocals {
                        model: model_data.into(),
                    }
                };

                encoder.push_graphics_constants(
                    pipeline_layout,
                    ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT,
                    0,
                    &locals.data()[..],
                );

                encoder.bind_vertex_buffers(pso::VertexBufferSet(vec![(model.vertex_buffer.buffer(), 0)]));
                encoder.bind_index_buffer(hal::buffer::IndexBufferView {
                    buffer: model.index_buffer.buffer(),
                    offset: 0,
                    index_type: hal::IndexType::U32,
                });
                encoder.draw_indexed(0..model.index_buffer.len(), 0, 0..1);
            }
        }
    }

    pub fn new(
        backbuffer: &hal::Backbuffer<B>,
        config: &Config,
        resources: &RLock<Resources>,
        dimensions: (u32, u32),
        dpi_factor: f32,
        device: Arc<Mutex<back::Device>>,
        limits: &hal::Limits,
        memory_types: &[hal::MemoryType],
        surface_format: f::Format,
        _depth_format: Option<f::Format>,
    ) -> Result<Self, Error> {
        let (width, height) = dimensions;

        let set_layout = {
            let device = device.lock().unwrap();
            device.create_descriptor_set_layout(&Image::<B>::descriptor_set_binding(
                ShaderStageFlags::FRAGMENT,
                0,
            )[..])
        };

        let pipeline_layout = {
            let device = device.lock().unwrap();
            device.create_pipeline_layout(Some(&set_layout), &[])
        };

        let render_pass = {
            let device = device.lock().unwrap();

            let attachment = pass::Attachment {
                format: Some(surface_format),
                ops: pass::AttachmentOps::new(pass::AttachmentLoadOp::Load, pass::AttachmentStoreOp::Store),
                stencil_ops: pass::AttachmentOps::DONT_CARE,
                layouts: i::Layout::Undefined .. i::Layout::Present,
            };

            let subpass = pass::SubpassDesc {
                colors: &[(0, i::Layout::ColorAttachmentOptimal)],
                depth_stencil: None,
                inputs: &[],
                preserves: &[],
            };

            let dependency = pass::SubpassDependency {
                passes: pass::SubpassRef::External .. pass::SubpassRef::Pass(0),
                stages: PipelineStage::COLOR_ATTACHMENT_OUTPUT .. PipelineStage::COLOR_ATTACHMENT_OUTPUT,
                accesses: i::Access::empty() .. (i::Access::COLOR_ATTACHMENT_READ | i::Access::COLOR_ATTACHMENT_WRITE),
            };

            device.create_render_pass(&[attachment], &[subpass], &[dependency])
        };

        let pipeline = {
            let device = device.lock().unwrap();

            let shader = Shader::load_from_config(config, resources, &ShaderKey::new("ui"))?;
            let vs_module = device.create_shader_module(&shader.vertex[..])
                .map_err(|_| RenderError::ShaderModuleFail("Vertex"))?;
            let fs_module = device.create_shader_module(&shader.fragment[..])
                .map_err(|_| RenderError::ShaderModuleFail("Fragment"))?;

            let pipeline = {
                let (vs_entry, fs_entry) = (
                    pso::EntryPoint::<B> {
                        entry: "main",
                        module: &vs_module,
                        specialization: &[],
                    },
                    pso::EntryPoint::<B> {
                        entry: "main",
                        module: &fs_module,
                        specialization: &[],
                    },
                );

                let shader_entries = pso::GraphicsShaderSet {
                    vertex: vs_entry,
                    hull: None,
                    domain: None,
                    geometry: None,
                    fragment: Some(fs_entry),
                };

                let subpass = Subpass { index: 0, main_pass: &render_pass };

                let mut pipeline_desc = pso::GraphicsPipelineDesc::new(
                    shader_entries,
                    Primitive::TriangleList,
                    pso::Rasterizer::FILL,
                    &pipeline_layout,
                    subpass,
                );
                pipeline_desc.blender.targets.push(pso::ColorBlendDesc(pso::ColorMask::ALL, pso::BlendState::ALPHA));

                pipeline_desc.vertex_buffers.push(pso::VertexBufferDesc {
                    stride: mem::size_of::<UiVertex>() as u32,
                    rate: 0,
                });

                pipeline_desc.attributes.extend(UiVertex::desc());

                device.create_graphics_pipeline(&pipeline_desc)?
            };

            device.destroy_shader_module(vs_module);
            device.destroy_shader_module(fs_module);

            pipeline
        };

        let mut desc_pool = {
            let device = device.lock().unwrap();

            device.create_descriptor_pool(1, &Image::<B>::descriptor_range()[..])
        };

        let desc_set = desc_pool.allocate_set(&set_layout);

        let framebuffers = match backbuffer {
            Backbuffer::Images(images) => {
                let device = device.lock().unwrap();

                let extent = i::Extent { width, height, depth: 1 };
                let pairs = images.iter()
                    .map(|image| {
                        let rtv = device.create_image_view(&image, i::ViewKind::D2, surface_format, Swizzle::NO, renderer::COLOR_RANGE.clone())?;
                        Ok(rtv)
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                let fbos = pairs.iter()
                    .map(|rtv| device.create_framebuffer(&render_pass, vec![rtv], extent))
                    .collect::<Result<Vec<_>, _>>().map_err(|_| RenderError::FramebufferCreation)?;

                fbos
            },
            Backbuffer::Framebuffer(_) => Err(RenderError::FramebufferCreation)?,
        };

        let locals = Buffer::<<Self as Pipe>::Locals, B>::new(device.clone(), 1, hal::buffer::Usage::UNIFORM, &memory_types).unwrap();

        let viewport = pso::Viewport {
            rect: pso::Rect {
                x: 0,
                y: 0,
                w: width as _,
                h: height as _,
            },
            depth: 0.0 .. 1.0,
        };

        let sampler = {
            let device = device.lock().unwrap();
            device.create_sampler(i::SamplerInfo::new(
                i::Filter::Linear,
                i::WrapMode::Clamp,
            ))
        };
        let sampler = Arc::new(Sampler::new(sampler));

        let (glyph_cache, cache_tex) = {
            let width = config.font_resolution * (dpi_factor as u32);
            let height = config.font_resolution * (dpi_factor as u32);

            const SCALE_TOLERANCE: f32 = 0.1;
            const POSITION_TOLERANCE: f32 = 0.1;

            let glyph_cache = rusttype::gpu_cache::CacheBuilder {
                width,
                height,
                scale_tolerance: SCALE_TOLERANCE,
                position_tolerance: POSITION_TOLERANCE,
                pad_glyphs: true,
            }.build();

            let data = vec![[0; 4]; (width * height) as usize];

            let (_, image) = Image::from_data(String::from("GlyphCache"), width, height, &data, limits, device.clone(), memory_types, sampler.clone()).unwrap();

            (glyph_cache, image)
        };

        {
            let device = device.lock().unwrap();

            device.write_descriptor_sets(cache_tex.descriptor_set(0, &desc_set));
        }

        Ok(Self {
            dimensions,
            device,
            dpi_factor,
            viewport,
            pipeline_layout,
            render_pass,
            pipeline,
            desc_set,
            framebuffers,
            locals,
            sampler,
            glyph_cache,
            cache_tex,
        })
    }
}
