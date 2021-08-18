use std::borrow::{Borrow, BorrowMut};
use std::convert::{AsMut, AsRef};
use std::ops::{Deref, DerefMut};

use ash::vk;

use crate::Destroyable;

/// Most common usecase for [`GuardedResource`]
///
/// Fine-grained RAII should be short-lived, making references preferred.
///
/// Note that `'r` can be `'static` when `Resource` is [`ash::Instance`] or [`ash::Device`].
pub type Guarded<'r, Resource> =
    GuardedResource<'static, Resource, &'r <Resource as Destroyable>::Destroyer>;

/// [`ScopeGuard`](https://docs.rs/scopeguard/1.1.0/scopeguard/struct.ScopeGuard.html) tailored
/// for Vulkan
///
/// When the [`GuardedResource`] is dropped, the contained `Resource` is destroyed, generally by
/// calling an appropriate method on the `Destroyer` (usually an [`&ash::Device`](ash::Device))
/// with `allocation_callbacks`. The contained resource can be accessed by dereferencing or
/// extracted with [`.take()`](Self::take). Application-specific types are supported if they
/// implement [`Destroyable`]. The [`Guarded`] alias is provided for the common use-case where
/// `Destroyer` is a reference.
///
/// ```
/// use ash::{prelude::VkResult, vk};
/// use ashpan::{DeviceExt, Guarded};
///
/// unsafe fn create_pipeline(device: &ash::Device) -> VkResult<Guarded<vk::Pipeline>> {
///     let pipeline_cache = unimplemented!();
///     let create_info = unimplemented!();
///
///     // Because the returned pipelines are wrapped in a GuardedResource,
///     // they don't leak when dropped by .map_err()
///     let pipelines = device
///         .create_guarded_graphics_pipelines(pipeline_cache, &[create_info], None)
///         .map_err(|(_, err)| err)?;
///
///     assert_eq!(pipelines.len(), 1);
///     let pipeline = pipelines.take()[0];
///     // This would also work:
///     // let pipeline = pipelines.pop().unwrap();
///
///     Ok(Guarded::new(pipeline, device, None))
/// }
/// ```
#[derive(Debug)]
pub struct GuardedResource<'alloc_cb, Resource, Destroyer>(
    // Invariant: The option is always Some, except possibly while being dropped.
    Option<ResourceAndDestroyer<'alloc_cb, Resource, Destroyer>>,
)
where
    Resource: Destroyable,
    Destroyer: Deref<Target = <Resource as Destroyable>::Destroyer>;

#[derive(Debug)]
struct ResourceAndDestroyer<'alloc_cb, Resource, Destroyer> {
    resource: Resource,
    destroyer: Destroyer,
    allocation_callbacks: Option<&'alloc_cb vk::AllocationCallbacks>,
}

impl<'alloc_cb, Resource, Destroyer> GuardedResource<'alloc_cb, Resource, Destroyer>
where
    Resource: Destroyable,
    Destroyer: Deref<Target = <Resource as Destroyable>::Destroyer>,
{
    /// Creates a [`GuardedResource`] to hold the passed `resource`. `destroyer` and
    /// `allocation_callbacks` are used during destruction.
    ///
    /// # Safety
    ///
    /// You must ensure that it is safe to destroy `resource` when the [`GuardedResource`] is
    /// dropped.
    pub unsafe fn new(
        resource: Resource,
        destroyer: Destroyer,
        allocation_callbacks: Option<&'alloc_cb vk::AllocationCallbacks>,
    ) -> Self {
        Self(Some(ResourceAndDestroyer {
            resource,
            destroyer,
            allocation_callbacks,
        }))
    }

    /// Extract the inner value without destroying it.
    ///
    /// ## Note
    ///
    /// Unlike
    /// [`ScopeGuard::into_inner`](https://docs.rs/scopeguard/1.1.0/scopeguard/struct.ScopeGuard.html#method.into_inner),
    /// this is a method because it's not intended to work with arbitrary types, so avoiding
    /// shadowing `.take()` is less important than convenience.
    pub fn take(mut self) -> Resource {
        self.0.take().unwrap().resource
    }
}

impl<'alloc_cb, Resource, Destroyer> AsRef<Resource>
    for GuardedResource<'alloc_cb, Resource, Destroyer>
where
    Resource: Destroyable,
    Destroyer: Deref<Target = <Resource as Destroyable>::Destroyer>,
{
    fn as_ref(&self) -> &Resource {
        &*self
    }
}

impl<'alloc_cb, Resource, Destroyer> AsMut<Resource>
    for GuardedResource<'alloc_cb, Resource, Destroyer>
where
    Resource: Destroyable,
    Destroyer: Deref<Target = <Resource as Destroyable>::Destroyer>,
{
    fn as_mut(&mut self) -> &mut Resource {
        &mut *self
    }
}

impl<'alloc_cb, Resource, Destroyer> Borrow<Resource>
    for GuardedResource<'alloc_cb, Resource, Destroyer>
where
    Resource: Destroyable,
    Destroyer: Deref<Target = <Resource as Destroyable>::Destroyer>,
{
    fn borrow(&self) -> &Resource {
        &*self
    }
}

impl<'alloc_cb, Resource, Destroyer> BorrowMut<Resource>
    for GuardedResource<'alloc_cb, Resource, Destroyer>
where
    Resource: Destroyable,
    Destroyer: Deref<Target = <Resource as Destroyable>::Destroyer>,
{
    fn borrow_mut(&mut self) -> &mut Resource {
        &mut *self
    }
}

impl<'alloc_cb, Resource, Destroyer> Deref for GuardedResource<'alloc_cb, Resource, Destroyer>
where
    Resource: Destroyable,
    Destroyer: Deref<Target = <Resource as Destroyable>::Destroyer>,
{
    type Target = Resource;

    fn deref(&self) -> &Self::Target {
        &self.0.as_ref().unwrap().resource
    }
}

impl<'alloc_cb, Resource, Destroyer> DerefMut for GuardedResource<'alloc_cb, Resource, Destroyer>
where
    Resource: Destroyable,
    Destroyer: Deref<Target = <Resource as Destroyable>::Destroyer>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.as_mut().unwrap().resource
    }
}

impl<'alloc_cb, Resource, Destroyer> Drop for GuardedResource<'alloc_cb, Resource, Destroyer>
where
    Resource: Destroyable,
    Destroyer: Deref<Target = <Resource as Destroyable>::Destroyer>,
{
    fn drop(&mut self) {
        if let Some(ResourceAndDestroyer {
            resource,
            destroyer,
            allocation_callbacks,
        }) = self.0.as_mut()
        {
            unsafe { resource.destroy_with(destroyer, *allocation_callbacks) }
        }
    }
}
