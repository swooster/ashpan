use ash::vk;

use crate::GuardedResource;

/// Extension trait adding guarded methods to [`ash::EntryCustom`]
pub trait EntryExt {
    /// Same as [ash::EntryCustom::create_instance] but returns guarded instance
    #[allow(clippy::missing_safety_doc)]
    unsafe fn create_guarded_instance<'a>(
        &self,
        create_info: &vk::InstanceCreateInfo,
        allocation_callbacks: Option<&'a vk::AllocationCallbacks>,
    ) -> Result<GuardedResource<'a, ash::Instance, &'static ()>, ash::InstanceError>;
}

impl<L> EntryExt for ash::EntryCustom<L> {
    unsafe fn create_guarded_instance<'a>(
        &self,
        create_info: &vk::InstanceCreateInfo,
        allocation_callbacks: Option<&'a vk::AllocationCallbacks>,
    ) -> Result<GuardedResource<'a, ash::Instance, &'static ()>, ash::InstanceError> {
        let instance = self.create_instance(create_info, allocation_callbacks)?;
        Ok(GuardedResource::new(instance, &(), allocation_callbacks))
    }
}
