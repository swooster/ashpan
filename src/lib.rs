#![doc(html_root_url = "https://docs.rs/ashpan/0.6.2")]
//! This crate provides RAII helpers for [`ash`]. In particular:
//!
//! * [`Guarded`]/[`GuardedResource`] is essentially a
//!   [`ScopeGuard`](https://docs.rs/scopeguard/1.1.0/scopeguard/struct.ScopeGuard.html)
//!   that selects an appropriate destructor automatically.
//! * [`DeviceExt`] (along with [`EntryExt`] and [`InstanceExt`]) provide convenience methods to
//!   create resources and wrap them in [`GuardedResource`]s.
//! * [`Destroyable`] allows you to extend the behavior of [`GuardedResource`].
//!
//! # Introduction
//!
//! When working with Vulkan, you generally want to group multiple resources together into a few
//! large structs. Unfortunately, errors during initialization can leak resources:
//!
//! ```
//! # use ash::{prelude::VkResult, vk};
//! #
//! # struct Resources {
//! #     render_pass: vk::RenderPass,
//! #     pipeline_layout: vk::PipelineLayout,
//! #     pipeline: vk::Pipeline,
//! # }
//! #
//! unsafe fn create_resources(device: &ash::Device) -> VkResult<Resources> {
//!     let render_pass = create_render_pass(device)?;
//!
//!     // BUG: Failure leaks render_pass
//!     let pipeline_layout = create_pipeline_layout(device)?;
//!
//!     // BUG: Failure leaks render_pass and pipeline_layout
//!     let pipeline = create_pipeline(device, render_pass, pipeline_layout)?;
//!
//!     Ok(Resources {
//!         render_pass,
//!         pipeline_layout,
//!         pipeline,
//!     })
//! }
//! # fn create_render_pass(_: &ash::Device) -> VkResult<vk::RenderPass> { unimplemented!() }
//! # fn create_pipeline_layout(_: &ash::Device) -> VkResult<vk::PipelineLayout> { unimplemented!() }
//! # fn create_pipeline(
//! #     _: &ash::Device,
//! #     _: vk::RenderPass,
//! #     _: vk::PipelineLayout,
//! # ) -> VkResult<vk::Pipeline> {
//! #     unimplemented!()
//! # }
//! ```
//!
//! It's straightforward to solve this with [`scopeguard`](https://docs.rs/scopeguard/), but it
//! tends to be a bit verbose and repetitive:
//!
//! ```
//! # use ash::{prelude::VkResult, vk};
//! use scopeguard::ScopeGuard;
//! # struct Resources {
//! #     render_pass: vk::RenderPass,
//! #     pipeline_layout: vk::PipelineLayout,
//! #     pipeline: vk::Pipeline,
//! # }
//!
//! type Guarded<'a, T> = ScopeGuard<(T, &'a ash::Device), fn((T, &'a ash::Device))>;
//!
//! unsafe fn create_resources(device: &ash::Device) -> VkResult<Resources> {
//!     let render_pass = create_render_pass(device)?;
//!     let pipeline_layout = create_pipeline_layout(device)?;
//!     let pipeline = create_pipeline(device, render_pass.0, pipeline_layout.0)?;
//!     Ok(Resources {
//!         render_pass: ScopeGuard::into_inner(render_pass).0,
//!         pipeline_layout: ScopeGuard::into_inner(pipeline_layout).0,
//!         pipeline: ScopeGuard::into_inner(pipeline).0,
//!     })
//! }
//!
//! unsafe fn create_render_pass(device: &ash::Device) -> VkResult<Guarded<vk::RenderPass>> {
//!     let create_info = unimplemented!();
//!     let render_pass = device.create_render_pass(create_info, None)?;
//!     Ok(scopeguard::guard(
//!         (render_pass, device),
//!         |(render_pass, device)| device.destroy_render_pass(render_pass, None),
//!     ))
//! }
//!
//! // fn create_pipeline_layout(...) { ... }
//! // fn create_pipeline(...) { ... }
//!
//! # fn create_pipeline_layout(_: &ash::Device) -> VkResult<Guarded<vk::PipelineLayout>> {
//! #     unimplemented!()
//! # }
//! # fn create_pipeline(
//! #     _: &ash::Device,
//! #     _: vk::RenderPass,
//! #     _: vk::PipelineLayout,
//! # ) -> VkResult<Guarded<vk::Pipeline>> {
//! #     unimplemented!()
//! # }
//! ```
//!
//! [`ashpan`](crate) reduces the friction of using [`scopeguard`](https://docs.rs/scopeguard/)
//! with [`ash`] by automatically selecting the destructor, passing the same
//! `allocation_callbacks` to the destructor that were used for resource creation and making
//! guarded resources convenient to extract:
//!
//! ```
//! # use ash::{prelude::VkResult, vk};
//! use ashpan::{DeviceExt, Guarded};
//! #
//! # struct Resources {
//! #     render_pass: vk::RenderPass,
//! #     pipeline_layout: vk::PipelineLayout,
//! #     pipeline: vk::Pipeline,
//! # }
//!
//! unsafe fn create_resources(device: &ash::Device) -> VkResult<Resources> {
//!     let render_pass = create_render_pass(device)?;
//!     let pipeline_layout = create_pipeline_layout(device)?;
//!     let pipeline = create_pipeline(device, *render_pass, *pipeline_layout)?;
//!     Ok(Resources {
//!         render_pass: render_pass.take(),
//!         pipeline_layout: pipeline_layout.take(),
//!         pipeline: pipeline.take(),
//!     })
//! }
//!
//! unsafe fn create_render_pass(device: &ash::Device) -> VkResult<Guarded<vk::RenderPass>> {
//!     let create_info = unimplemented!();
//!     device.create_guarded_render_pass(create_info, None)
//! }
//!
//! // fn create_pipeline_layout(...) { ... }
//! // fn create_pipeline(...) { ... }
//!
//! # fn create_pipeline_layout(_: &ash::Device) -> VkResult<Guarded<vk::PipelineLayout>> {
//! #     unimplemented!()
//! # }
//! # fn create_pipeline(
//! #     _: &ash::Device,
//! #     _: vk::RenderPass,
//! #     _: vk::PipelineLayout,
//! # ) -> VkResult<Guarded<vk::Pipeline>> {
//! #     unimplemented!()
//! # }
//! ```
//!
//! It's also possible to extend [`Guarded`] to handle application-specific types:
//!
//! ```
//! # use ash::{prelude::VkResult, vk};
//! use ashpan::{Destroyable, DeviceExt, Guarded};
//! #
//! # struct Resources {
//! #     render_pass: vk::RenderPass,
//! #     pipeline_layout: vk::PipelineLayout,
//! #     pipeline: vk::Pipeline,
//! # }
//!
//! impl Destroyable for Resources {
//!     type Destroyer = ash::Device;
//!
//!     unsafe fn destroy_with(
//!         &mut self,
//!         device: &ash::Device,
//!         allocation_callbacks: Option<&vk::AllocationCallbacks>,
//!     ) {
//!         device.destroy_pipeline(self.pipeline, allocation_callbacks);
//!         device.destroy_pipeline_layout(self.pipeline_layout, allocation_callbacks);
//!         device.destroy_render_pass(self.render_pass, allocation_callbacks);
//!     }
//! }
//!
//! // Elsewhere...
//! unsafe fn create_resources(device: &ash::Device) -> VkResult<Guarded<Resources>> {
//!     let resources = unimplemented!();
//!     Ok(Guarded::new(resources, device, None))
//! }
//! ```

mod destroy;
mod device;
mod entry;
mod guarded;
mod instance;

pub use destroy::Destroyable;
pub use device::DeviceExt;
pub use entry::EntryExt;
pub use guarded::{Guarded, GuardedResource};
pub use instance::InstanceExt;

#[cfg(test)]
mod tests {
    use crate::{DeviceExt, EntryExt, InstanceExt};

    #[test]
    fn sanity_check_extension_trait_compilation() {
        if true {
            return;
        }

        // Circumvent Rust's knowledge that unimplemented!() never returns.
        fn unimplemented<T>() -> T {
            unimplemented!()
        }

        unsafe {
            let entry = ash::Entry::load().unwrap();
            let instance = entry
                .create_guarded_instance(unimplemented(), None)
                .unwrap();
            let device = instance
                .create_guarded_device(unimplemented(), unimplemented(), None)
                .unwrap();
            let _semaphore = (&*device)
                .create_guarded_semaphore(unimplemented(), None)
                .unwrap();
        };
    }
}
