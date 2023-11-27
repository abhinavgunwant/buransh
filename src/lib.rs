use std::iter;

use winit::{
    event::*,
    event_loop::EventLoop,
    window::{ WindowBuilder, Window },
    dpi::{ LogicalSize, PhysicalSize },
};
use wgpu::{
    Surface, Device, Queue, SurfaceConfiguration, Instance, InstanceDescriptor,
    Backends, RequestAdapterOptions, PowerPreference, DeviceDescriptor,
    Features, Limits, TextureUsages, SurfaceError, TextureViewDescriptor,
    CommandEncoderDescriptor, RenderPassDescriptor, RenderPassColorAttachment,
    Operations, LoadOp, Color, StoreOp,
};
use log::{ info, error };

struct GraphicsState {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
    window: Window,
}

impl GraphicsState {
    pub async fn new(window: Window) -> Self {
        let size = window.inner_size();

        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance.request_adapter(
            &RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &DeviceDescriptor {
                features: Features::empty(),
                limits: if cfg!(target_arch = "wasm32") {
                    Limits::downlevel_webgl2_defaults()
                } else {
                    Limits::default()
                },
                label: None,
            },
            None,
        ).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
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

    fn update(&mut self) {}

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
                            r: 0.1,
                            g: 0.2,
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

    let mut graphics_state: GraphicsState = GraphicsState::new(window).await;

    info!("Starting up window.");

    let _ = event_loop.run(move | event, elwt | {
        match event {
            Event::WindowEvent {
                window_id, event: win_event
            } if window_id == graphics_state.window().id() => if !graphics_state.input(&win_event) {
                match win_event {
                    WindowEvent::CloseRequested => { elwt.exit(); }

                    WindowEvent::Resized(physical_size) => {
                        graphics_state.resize(physical_size);
                    }

                    WindowEvent::RedrawRequested => {
                        graphics_state.update();

                        match graphics_state.render() {
                            Ok(_) => {}

                            Err(SurfaceError::Lost) => {
                                graphics_state.resize(graphics_state.size);
                            },

                            Err(SurfaceError::OutOfMemory) => {
                                elwt.exit();
                            }

                            Err(e) => { error!("{:?}", e); }
                        }
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

