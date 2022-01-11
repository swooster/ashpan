use ash::{
    extensions::{ext, khr, nv},
    vk,
};

/// Indicates that a type is destroyable
///
/// Vulkan resources generally need to be created and destroyed via an [`ash::Device`] or a Vulkan
/// extension. The [`Destroyable`] trait provides a common interface, allowing compile-time
/// selection of an appropriate destructor based on the resource type.
///
/// Implementing [`Destroyable`] for custom types makes them work with
/// [`GuardedResource`](crate::GuardedResource):
///
/// ```
/// # use ash::{prelude::VkResult, vk};
/// use ashpan::{Destroyable, DeviceExt, Guarded};
///
/// struct Resources {
///     render_pass: vk::RenderPass,
///     pipeline_layout: vk::PipelineLayout,
///     pipeline: vk::Pipeline,
/// }
///
/// impl Resources {
///     unsafe fn new(device: &ash::Device) -> VkResult<Guarded<Self>> {
///         let resources = unimplemented!();
///         Ok(Guarded::new(resources, device, None))
///     }
/// }
///
/// impl Destroyable for Resources {
///     type Destroyer = ash::Device;
///
///     unsafe fn destroy_with(
///         &mut self,
///         device: &ash::Device,
///         allocation_callbacks: Option<&vk::AllocationCallbacks>,
///     ) {
///         device.destroy_pipeline(self.pipeline, allocation_callbacks);
///         device.destroy_pipeline_layout(self.pipeline_layout, allocation_callbacks);
///         device.destroy_render_pass(self.render_pass, allocation_callbacks);
///     }
/// }
///
/// // Elsewhere...
/// unsafe fn build_and_frob_resources(device: &ash::Device) -> VkResult<()> {
///     let resources = Resources::new(device)?;
///     frob_resources(&*resources)?;
///     Ok(())
/// }
///
/// fn frob_resources(resources: &Resources) -> VkResult<()> {
///     unimplemented!()
/// }
/// ```
pub trait Destroyable {
    /// The type that performs the destruction of the `Destroyable`
    type Destroyer: ?Sized;

    /// Destroys `self` via `destroyer` with `allocation_callbacks`.
    ///
    /// # Safety
    ///
    /// Depends on the resource type; see Vulkan spec for details.
    unsafe fn destroy_with(
        &mut self,
        destroyer: &Self::Destroyer,
        allocation_callbacks: Option<&vk::AllocationCallbacks>,
    );
}

impl Destroyable for ash::Instance {
    type Destroyer = ();

    unsafe fn destroy_with(
        &mut self,
        _destroyer: &(),
        allocation_callbacks: Option<&vk::AllocationCallbacks>,
    ) {
        self.destroy_instance(allocation_callbacks);
    }
}

impl Destroyable for ash::Device {
    type Destroyer = ();

    unsafe fn destroy_with(
        &mut self,
        _destroyer: &(),
        allocation_callbacks: Option<&vk::AllocationCallbacks>,
    ) {
        self.destroy_device(allocation_callbacks);
    }
}

macro_rules! destroyable {
    ($destroy:ident, $Resource:ty) => {
        impl Destroyable for $Resource {
            type Destroyer = ash::Device;

            unsafe fn destroy_with(
                &mut self,
                device: &ash::Device,
                allocation_callbacks: Option<&vk::AllocationCallbacks>,
            ) {
                device.$destroy(*self, allocation_callbacks);
            }
        }
    };
}

// Version 1.0
destroyable!(destroy_buffer, vk::Buffer);
destroyable!(destroy_buffer_view, vk::BufferView);
destroyable!(destroy_command_pool, vk::CommandPool);
destroyable!(destroy_descriptor_pool, vk::DescriptorPool);
destroyable!(destroy_descriptor_set_layout, vk::DescriptorSetLayout);
destroyable!(free_memory, vk::DeviceMemory);
destroyable!(destroy_event, vk::Event);
destroyable!(destroy_fence, vk::Fence);
destroyable!(destroy_framebuffer, vk::Framebuffer);
destroyable!(destroy_image, vk::Image);
destroyable!(destroy_image_view, vk::ImageView);
destroyable!(destroy_pipeline, vk::Pipeline);
destroyable!(destroy_pipeline_layout, vk::PipelineLayout);
destroyable!(destroy_pipeline_cache, vk::PipelineCache);
destroyable!(destroy_query_pool, vk::QueryPool);
destroyable!(destroy_render_pass, vk::RenderPass);
destroyable!(destroy_sampler, vk::Sampler);
destroyable!(destroy_semaphore, vk::Semaphore);
destroyable!(destroy_shader_module, vk::ShaderModule);
// Version 1.1
destroyable!(
    destroy_descriptor_update_template,
    vk::DescriptorUpdateTemplate
);
destroyable!(destroy_sampler_ycbcr_conversion, vk::SamplerYcbcrConversion);

// TODO: Look for ways to implement something vaguely equivalent to:
//     Destroyable<Destroyer=(&ash::Device, vk::CommandPool)> vk::CommandBuffer
//     Destroyable<Destroyer=(&ash::Device, vk::DescriptorPool)> vk::DescriptorSet

macro_rules! destroyable_ext {
    ($Destroyer:ty, $destroy:ident, $Resource:ty) => {
        impl Destroyable for $Resource {
            type Destroyer = $Destroyer;

            unsafe fn destroy_with(
                &mut self,
                destroyer: &Self::Destroyer,
                allocation_callbacks: Option<&vk::AllocationCallbacks>,
            ) {
                destroyer.$destroy(*self, allocation_callbacks);
            }
        }
    };
}

destroyable_ext!(
    khr::AccelerationStructure,
    destroy_acceleration_structure,
    vk::AccelerationStructureKHR
);
destroyable_ext!(
    nv::RayTracing,
    destroy_acceleration_structure,
    vk::AccelerationStructureNV
);
destroyable_ext!(
    ext::DebugUtils,
    destroy_debug_utils_messenger,
    vk::DebugUtilsMessengerEXT
);
destroyable_ext!(
    khr::DeferredHostOperations,
    destroy_deferred_operation,
    vk::DeferredOperationKHR
);
destroyable_ext!(khr::Surface, destroy_surface, vk::SurfaceKHR);
destroyable_ext!(khr::Swapchain, destroy_swapchain, vk::SwapchainKHR);

// TODO: Figure out the following:
//     CuFunctionNVX
//     CuModuleNVX
//     DisplayKHR
//     DisplayModeKHR
//     IndirectCommandsLayoutNV
//     PerformanceConfigurationINTEL
//     PrivateDataSlotEXT
//     ValidationCacheEXT
//     VideoSessionKHR
//     VideoSessionParametersKHR

impl<Resource: Destroyable> Destroyable for Vec<Resource> {
    type Destroyer = <Resource as Destroyable>::Destroyer;

    unsafe fn destroy_with(
        &mut self,
        destroyer: &Self::Destroyer,
        allocation_callbacks: Option<&vk::AllocationCallbacks>,
    ) {
        for mut resource in self.drain(..) {
            resource.destroy_with(destroyer, allocation_callbacks);
        }
    }
}

impl<Resource: Destroyable, const N: usize> Destroyable for [Resource; N] {
    type Destroyer = <Resource as Destroyable>::Destroyer;

    unsafe fn destroy_with(
        &mut self,
        destroyer: &Self::Destroyer,
        allocation_callbacks: Option<&vk::AllocationCallbacks>,
    ) {
        for resource in self {
            resource.destroy_with(destroyer, allocation_callbacks);
        }
    }
}
