use vulkano::sync::now;

use vulkano::device::{  Device, 
                        DeviceCreateInfo, 
                        DeviceExtensions, 
                        QueueCreateInfo, 
                        Queue, 
                        QueueFlags };

use vulkano::image::{ ImageUsage };
use vulkano::image::view::{ ImageView };

use vulkano::sync::GpuFuture;
use vulkano::command_buffer::{
        AutoCommandBufferBuilder, 
        CommandBufferUsage, 
        RenderPassBeginInfo, 
        SubpassContents,
        SubpassEndInfo,
    };

use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;

use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass};
use vulkano::swapchain::{
        Surface, 
        SurfaceInfo, 
        Swapchain, 
        SwapchainCreateInfo, 
        PresentMode,
        SwapchainPresentInfo
    };

use vulkano::format::ClearValue;

use vulkano::{
        Version, 
        VulkanLibrary 
    };

use std::sync::Arc;

pub struct Renderer {
    inst: RenderInstance,
}

use winit::window::Window;

impl Renderer {
    pub fn new( window: Arc<Window> ) -> Self {
        // build RenderInstance
        Self { inst: RenderInstance::new( window ) }
    }

    // expose *intent*, not guts
    pub fn request_redraw(&self) { }

    pub fn render(&mut self) {
        let inst = &mut self.inst;
        let rcx = inst.rcx.as_ref().expect("RenderContext not initialized");

        // 1) Clean up GPU work from previous frames
        if let Some(fut) = inst.previous_frame_end.as_mut() {
            fut.cleanup_finished();
        }

        // 2) Acquire the next swapchain image to render into
        let (image_index, _suboptimal, acquire_future) =
            match swapchain::acquire_next_image(rcx.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    // window resized / swapchain invalid; recreate later
                    return;
                }
                Err(e) => panic!("Failed to acquire next image: {e:?}"),
            };

        // 3) Record command buffer: begin render pass with a clear color, then end.
        let framebuffer = rcx.frame_buffers[image_index as usize].clone();

        let mut builder = AutoCommandBufferBuilder::primary(
            &inst.command_buffer_allocator,
            inst.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some(ClearValue::Float([0.1, 0.1, 0.2, 1.0]))], // bluish
                    ..RenderPassBeginInfo::framebuffer(framebuffer)
                },
                SubpassContents::Inline,
            )
            .unwrap();

        // (Later: bind pipeline + draw here)

        builder.end_render_pass().unwrap();

        let command_buffer = builder.build().unwrap();

        // 4) Submit + present, chaining futures correctly
        let previous = inst.previous_frame_end.take().unwrap();

        let future = previous
            .join(acquire_future)
            .then_execute(inst.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                inst.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(rcx.swapchain.clone(), image_index),
            )
            .then_signal_fence_and_flush();

        inst.previous_frame_end = Some(match future {
            Ok(f) => f.boxed(),
            Err(sync::FlushError::OutOfDate) => {
                // swapchain became invalid during present; recreate later
                sync::now(inst.device.clone()).boxed()
            }
            Err(e) => {
                eprintln!("Flush error: {e:?}");
                sync::now(inst.device.clone()).boxed()
            }
        });
    }
}

struct RenderInstance {
    instance: Arc<Instance>,
    surface: Arc<Surface>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    command_buffer_allocator: StandardCommandBufferAllocator,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    rcx: Option<RenderContext>,
}

struct RenderContext {
    swapchain: Arc<Swapchain>,
    image_views: Vec<Arc<ImageView>>,
    render_pass: Arc<RenderPass>,
    frame_buffers: Vec<Arc<Framebuffer>>
}

impl RenderContext {
    fn new( device: Arc<Device>,
            surface: Arc<Surface>,
            create_info: SwapchainCreateInfo ) -> Self {

        let ( swapchain, images)  = 
            Swapchain::new ( 
                device.clone(),
                surface.clone(),
                create_info )
            .unwrap();

        let image_views: Vec<_> = images
            .iter()
            .map(|img| ImageView::new_default(img.clone()).unwrap())
            .collect();

        let swapchain_format = swapchain.image_format();

        let render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    format: swapchain_format,
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        ).unwrap();

        let frame_buffers: Vec<Arc<Framebuffer>> = image_views
            .iter()
            .map(|view| {
                Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![view.clone()],
                        ..Default::default()
                    },
                )
                .unwrap()
            })
            .collect();


        RenderContext {
            swapchain,
            image_views,
            render_pass,
            frame_buffers
        }
    }
}

impl RenderInstance {
    pub fn new( window: Arc<Window> )-> Self {
        let instance = {
            let library = VulkanLibrary::new().unwrap();
            let extensions = Surface::required_extensions(window.as_ref()).unwrap();

            Instance::new(
                library,
                InstanceCreateInfo {
                    enabled_extensions: extensions,
                    max_api_version: Some(Version::V1_1),
                    ..Default::default()
                },
            )
            .unwrap()
        };

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        let surface = Surface::from_window(instance.clone(), window.clone()).unwrap();

        let mut physical_devices = instance
            .enumerate_physical_devices()
            .unwrap()
            .filter(|p| {
                // Some devices may not support the extensions or features that your application,
                // or report properties and limits that are not sufficient for your application.
                // These should be filtered out here.
                p.supported_extensions().contains(&device_extensions)
            });

        let physical_device = physical_devices
            .find(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .any(|(i, q)| {
                        q.queue_flags.contains(QueueFlags::GRAPHICS)
                            && p.surface_support(i as u32, &surface).unwrap_or(false)
                    })
            })
        .expect("No suitable physical device found");

        println!(
            "using device: {} ({:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        let queue_family_index: u32 = physical_device
            .queue_family_properties()
            .iter()
            .enumerate()
            .position(|(i, q)| {
                q.queue_flags.contains(QueueFlags::GRAPHICS)
                    && physical_device.surface_support(i as u32, &surface).unwrap_or(false)
            })
            .expect("No graphics+present queue family found") as u32;

        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                queue_create_infos: vec![
                    QueueCreateInfo {
                        queue_family_index,
                        ..Default::default()
                    }
                ],
                ..Default::default()
            },
        ).expect("Failed to create logical device");

        let queue = queues.next().expect("No queue returned by Device::new");

        let window_size = window.inner_size();
        let image_extent = [window_size.width, window_size.height];

        let formats = physical_device
            .surface_formats(&surface, SurfaceInfo::default())
            .unwrap();

        let (image_format, _color_space) = formats
            .iter()
            .cloned()
            .find(|(f, _)| *f == vulkano::format::Format::B8G8R8A8_SRGB)
            .unwrap_or(formats[0]);

        println!("Supported surface formats:");
        for (format, color_space) in formats.clone() {
            println!("  {:?} / {:?}", format, color_space);
        }

        let caps = physical_device.surface_capabilities(&surface, Default::default()).unwrap();
        let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();

        let swapchaininfo = SwapchainCreateInfo {
            min_image_count: caps.min_image_count,
            image_format: image_format,
            image_extent,
            image_usage: ImageUsage::COLOR_ATTACHMENT, // we will render into it
            composite_alpha,
            present_mode: PresentMode::Fifo, // vsync; guaranteed supported
            ..Default::default()
        };

     //   let command_buffer_allocator = None;
        let rcx = Some ( 
            RenderContext::new( 
                device.clone(), 
                surface.clone(),
    swapchaininfo
            ));
        
        let command_buffer_allocator =
            StandardCommandBufferAllocator::new(device.clone(), Default::default());

        // This future represents “nothing has been submitted yet”, and it’s the standard starting point.
        let previous_frame_end = Some(vulkano::sync::now(device.clone()).boxed());


        RenderInstance {
            instance,
            surface,
            device,
            queue,
            command_buffer_allocator,
            previous_frame_end,
            rcx,
        }

    }
}