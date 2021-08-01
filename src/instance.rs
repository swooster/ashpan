use ash::{prelude::VkResult, vk};

use crate::GuardedResource;

/// Extension trait adding guarded methods to [`ash::Instance`]
pub trait InstanceExt {
    /// Same as [ash::Instance::create_device] but returns guarded device
    #[allow(clippy::missing_safety_doc)]
    unsafe fn create_guarded_device<'a>(
        &self,
        physical_device: vk::PhysicalDevice,
        create_info: &vk::DeviceCreateInfo,
        allocation_callbacks: Option<&'a vk::AllocationCallbacks>,
    ) -> VkResult<GuardedResource<'a, ash::Device, &'static ()>>;
}

impl InstanceExt for ash::Instance {
    unsafe fn create_guarded_device<'a>(
        &self,
        physical_device: vk::PhysicalDevice,
        create_info: &vk::DeviceCreateInfo,
        allocation_callbacks: Option<&'a vk::AllocationCallbacks>,
    ) -> VkResult<GuardedResource<'a, ash::Device, &'static ()>> {
        let device = self.create_device(physical_device, create_info, allocation_callbacks)?;
        Ok(GuardedResource::new(device, &(), allocation_callbacks))
    }
}
