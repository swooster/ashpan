use ash::{prelude::VkResult, vk};

use crate::GuardedResource;

/// Extension trait adding guarded methods to [`ash::Entry`]
pub trait EntryExt {
    /// Same as [ash::Entry::create_instance] but returns guarded instance
    #[allow(clippy::missing_safety_doc)]
    unsafe fn create_guarded_instance<'a>(
        &self,
        create_info: &vk::InstanceCreateInfo,
        allocation_callbacks: Option<&'a vk::AllocationCallbacks>,
    ) -> VkResult<GuardedResource<'a, ash::Instance, &'static ()>>;
}

impl EntryExt for ash::Entry {
    unsafe fn create_guarded_instance<'a>(
        &self,
        create_info: &vk::InstanceCreateInfo,
        allocation_callbacks: Option<&'a vk::AllocationCallbacks>,
    ) -> VkResult<GuardedResource<'a, ash::Instance, &'static ()>> {
        let instance = self.create_instance(create_info, allocation_callbacks)?;
        Ok(GuardedResource::new(instance, &(), allocation_callbacks))
    }
}
