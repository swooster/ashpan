use std::ops::Deref;

use ash::{prelude::VkResult, vk};

use crate::GuardedResource;

macro_rules! declaration {
    ($name:ident, $create:expr, $CreateInfo:ty, $Resource:ty,) => {
        #[doc = concat!(
            "Same as [`", stringify!($create), "`](ash::Device::", stringify!($create), ") but ",
            "returns guarded [`", stringify!($Resource), "`]."
        )]
        unsafe fn $name<'a>(
            &self,
            create_info: &$CreateInfo,
            allocation_callbacks: Option<&'a vk::AllocationCallbacks>,
        ) -> VkResult<GuardedResource<'a, $Resource, Self>>;
    };
}

macro_rules! definition {
    ($name:ident, $create:ident, $CreateInfo:ty, $Resource:ty,) => {
        unsafe fn $name<'a>(
            &self,
            create_info: &$CreateInfo,
            allocation_callbacks: Option<&'a vk::AllocationCallbacks>,
        ) -> VkResult<GuardedResource<'a, $Resource, Self>> {
            let resource = self.$create(create_info, allocation_callbacks)?;
            Ok(GuardedResource::new(
                resource,
                self.clone(),
                allocation_callbacks,
            ))
        }
    };
}

macro_rules! device_methods {
    ($method:ident) => {
        $method!(
            allocate_guarded_memory,
            allocate_memory,
            vk::MemoryAllocateInfo,
            vk::DeviceMemory,
        );

        $method!(
            create_guarded_buffer,
            create_buffer,
            vk::BufferCreateInfo,
            vk::Buffer,
        );

        $method!(
            create_guarded_buffer_view,
            create_buffer_view,
            vk::BufferViewCreateInfo,
            vk::BufferView,
        );

        $method!(
            create_guarded_command_pool,
            create_command_pool,
            vk::CommandPoolCreateInfo,
            vk::CommandPool,
        );

        $method!(
            create_guarded_descriptor_pool,
            create_descriptor_pool,
            vk::DescriptorPoolCreateInfo,
            vk::DescriptorPool,
        );

        $method!(
            create_guarded_descriptor_set_layout,
            create_descriptor_set_layout,
            vk::DescriptorSetLayoutCreateInfo,
            vk::DescriptorSetLayout,
        );

        $method!(
            create_guarded_event,
            create_event,
            vk::EventCreateInfo,
            vk::Event,
        );

        $method!(
            create_guarded_fence,
            create_fence,
            vk::FenceCreateInfo,
            vk::Fence,
        );

        $method!(
            create_guarded_framebuffer,
            create_framebuffer,
            vk::FramebufferCreateInfo,
            vk::Framebuffer,
        );

        $method!(
            create_guarded_image,
            create_image,
            vk::ImageCreateInfo,
            vk::Image,
        );

        $method!(
            create_guarded_image_view,
            create_image_view,
            vk::ImageViewCreateInfo,
            vk::ImageView,
        );

        $method!(
            create_guarded_pipeline_layout,
            create_pipeline_layout,
            vk::PipelineLayoutCreateInfo,
            vk::PipelineLayout,
        );

        $method!(
            create_guarded_pipeline_cache,
            create_pipeline_cache,
            vk::PipelineCacheCreateInfo,
            vk::PipelineCache,
        );

        $method!(
            create_guarded_query_pool,
            create_query_pool,
            vk::QueryPoolCreateInfo,
            vk::QueryPool,
        );

        $method!(
            create_guarded_render_pass,
            create_render_pass,
            vk::RenderPassCreateInfo,
            vk::RenderPass,
        );

        $method!(
            create_guarded_sampler,
            create_sampler,
            vk::SamplerCreateInfo,
            vk::Sampler,
        );

        $method!(
            create_guarded_semaphore,
            create_semaphore,
            vk::SemaphoreCreateInfo,
            vk::Semaphore,
        );

        $method!(
            create_guarded_shader_module,
            create_shader_module,
            vk::ShaderModuleCreateInfo,
            vk::ShaderModule,
        );

        // v1.1

        $method!(
            create_guarded_descriptor_update_template,
            create_descriptor_update_template,
            vk::DescriptorUpdateTemplateCreateInfo,
            vk::DescriptorUpdateTemplate,
        );

        $method!(
            create_guarded_sampler_ycbcr_conversion,
            create_sampler_ycbcr_conversion,
            vk::SamplerYcbcrConversionCreateInfo,
            vk::SamplerYcbcrConversion,
        );

        // v1.2

        $method!(
            create_guarded_render_pass2,
            create_render_pass2,
            vk::RenderPassCreateInfo2,
            vk::RenderPass,
        );
    };
}

type PipelinesResult<T> = Result<T, (T, vk::Result)>;

/// Extension trait adding guarded methods to [`ash::Device`]
#[allow(clippy::missing_safety_doc)]
pub trait DeviceExt: Sized + Deref<Target = ash::Device> {
    device_methods!(declaration);

    /// Same as [`create_graphics_pipelines`](ash::Device::create_graphics_pipelines) but returns
    /// guarded [`vk::Pipeline`]s.
    unsafe fn create_guarded_graphics_pipelines<'a>(
        &self,
        pipeline_cache: vk::PipelineCache,
        create_infos: &[vk::GraphicsPipelineCreateInfo],
        allocation_callbacks: Option<&'a vk::AllocationCallbacks>,
    ) -> PipelinesResult<GuardedResource<'a, Vec<vk::Pipeline>, Self>>;

    /// Same as [`create_compute_pipelines`](ash::Device::create_compute_pipelines) but returns
    /// guarded [`vk::Pipeline`]s.
    unsafe fn create_guarded_compute_pipelines<'a>(
        &self,
        pipeline_cache: vk::PipelineCache,
        create_infos: &[vk::ComputePipelineCreateInfo],
        allocation_callbacks: Option<&'a vk::AllocationCallbacks>,
    ) -> PipelinesResult<GuardedResource<'a, Vec<vk::Pipeline>, Self>>;

    // TODO: allocate_guarded_command_buffers
    // TODO: allocate_guarded_descriptor_sets
}

impl<DeviceRef> DeviceExt for DeviceRef
where
    DeviceRef: Clone + Deref<Target = ash::Device>,
{
    device_methods!(definition);

    unsafe fn create_guarded_graphics_pipelines<'a>(
        &self,
        pipeline_cache: vk::PipelineCache,
        create_infos: &[vk::GraphicsPipelineCreateInfo],
        allocation_callbacks: Option<&'a vk::AllocationCallbacks>,
    ) -> PipelinesResult<GuardedResource<'a, Vec<vk::Pipeline>, Self>> {
        let guard = |pipelines| GuardedResource::new(pipelines, self.clone(), allocation_callbacks);

        self.create_graphics_pipelines(pipeline_cache, create_infos, allocation_callbacks)
            .map(guard)
            .map_err(|(pipelines, result)| (guard(pipelines), result))
    }

    unsafe fn create_guarded_compute_pipelines<'a>(
        &self,
        pipeline_cache: vk::PipelineCache,
        create_infos: &[vk::ComputePipelineCreateInfo],
        allocation_callbacks: Option<&'a vk::AllocationCallbacks>,
    ) -> PipelinesResult<GuardedResource<'a, Vec<vk::Pipeline>, Self>> {
        let guard = |pipelines| GuardedResource::new(pipelines, self.clone(), allocation_callbacks);

        self.create_compute_pipelines(pipeline_cache, create_infos, allocation_callbacks)
            .map(guard)
            .map_err(|(pipelines, result)| (guard(pipelines), result))
    }
}
