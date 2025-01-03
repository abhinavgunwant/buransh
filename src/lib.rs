use std::iter;

use winit::{
    dpi::{ LogicalSize, PhysicalSize }, event::*, event_loop::EventLoop, keyboard::{KeyCode, PhysicalKey}, window::{ Window, WindowBuilder }
};
use wgpu::{
    Surface, Device, Queue, SurfaceConfiguration, Instance, InstanceDescriptor,
    Backends, RequestAdapterOptions, PowerPreference, DeviceDescriptor,
    Features, Limits, TextureUsages, SurfaceError, TextureViewDescriptor,
    CommandEncoderDescriptor, RenderPassDescriptor, RenderPassColorAttachment,
    Operations, LoadOp, Color, StoreOp,
};
use log::{ info, error };

enum UpdateType {
    Pos((u16, u16)),
    NONE,
}

struct GraphicsState<'a> {
    surface: Surface<'a>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
    window: &'a Window,
    pos: (u16, u16),
}

impl<'a> GraphicsState<'a> {
    pub async fn new(window: &'a Window) -> GraphicsState<'a> {
        let size = window.inner_size();

        let instance = Instance::new(InstanceDescriptor {
            #[cfg(not(target_arch="wasm32"))]
            backends: Backends::PRIMARY,
            #[cfg(target_arch="wasm32")]
            backends: Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance.request_adapter(
            &RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &DeviceDescriptor {
                required_features: Features::empty(),
                required_limits: if cfg!(target_arch = "wasm32") {
                    Limits::downlevel_webgl2_defaults()
                } else {
                    Limits::default()
                },
                label: None,
                memory_hints: Default::default(),
            },
            None,
        ).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            pos: (0, 0),
        }
    }

    pub fn window(&self) -> &Window { &self.window }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self, update_type: UpdateType) {
        match update_type {
            UpdateType::Pos(pos) => self.pos = pos,
            UpdateType::NONE => {}
        }
    }

    fn render(&mut self) -> Result<(), SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let _render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            // r: 0.1,
                            r: self.pos.0 as f64 / self.size.width as f64,
                            // g: 0.2,
                            g: self.pos.1 as f64 / self.size.height as f64,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub async fn run() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    let window = WindowBuilder::new()
        .with_title("Buransh test")
        .with_inner_size(LogicalSize::new(512.0, 512.0))
        .build(&event_loop).unwrap();

    let mut graphics_state: GraphicsState = GraphicsState::new(&window).await;

    info!("Starting up window.");

    let _ = event_loop.run(move | event, control_flow | {
        match event {
            Event::WindowEvent {
                window_id, ref event
            } if window_id == graphics_state.window().id() => if !graphics_state.input(event) {
                match event {
                    WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                        event: KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            ..
                        },
                        ..
                    } => { control_flow.exit(); }

                    WindowEvent::Resized(physical_size) => {
                        info!("resizing");
                        graphics_state.resize(*physical_size);
                    }

                    WindowEvent::RedrawRequested => {
                        graphics_state.window().request_redraw();

                        // if !surface_configured {
                        //     return;
                        // }

                        graphics_state.update(UpdateType::NONE);

                        match graphics_state.render() {
                            Ok(_) => {}

                            Err(SurfaceError::Lost) => {
                                graphics_state.resize(graphics_state.size);
                            },

                            Err(SurfaceError::OutOfMemory) => {
                                error!("Out of memory!");
                                control_flow.exit();
                            }

                            Err(SurfaceError::Timeout) => {
                                error!("Surface Timeout!");
                            }

                            Err(SurfaceError::Outdated) => {
                                error!("Surface Outdated!");
                            }
                        }
                    }

                    WindowEvent::CursorMoved {
                        device_id,
                        position,
                    } => {
                        error!("position: x: {}, y: {}", position.x, position.y);
                        graphics_state.update(UpdateType::Pos((position.x as u16, position.y as u16)));
                    }

                    // WindowEvent::MainEventsCleared => {
                    //     graphics_state.window().request_redraw();
                    // }

                    // WindowEvent::ScaleFactorChanged { inner_size_writer, .. } => {
                    //     state.resize(**new_inner_size);
                    // }
                    _ => {}
                }
            }


            _ => {}
        }
    });
}

