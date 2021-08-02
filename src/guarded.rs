use std::borrow::{Borrow, BorrowMut};
use std::convert::{AsMut, AsRef};
use std::ops::{Deref, DerefMut};

use ash::vk;

use crate::Destroyable;

/// Most common usecase for [`GuardedResource`]
///
/// The vast majority of resource types are created and destroyed with [`ash::Device`], and
/// fine-grained RAII should be short-lived, making references preferred.
pub type Guarded<'a, Resource> = GuardedResource<'static, Resource, &'a ash::Device>;

/// [`ScopeGuard`](https://docs.rs/scopeguard/1.1.0/scopeguard/struct.ScopeGuard.html) tailored
/// for Vulkan
///
/// When the [`GuardedResource`] is dropped, the contained `Resource` is destroyed, generally by
/// calling an appropriate method on the `Context` (usually an [`&ash::Device`](ash::Device)) with
/// `allocation_callbacks`. The contained resource can be accessed by dereferencing or extracted
/// with [`.take()`](Self::take). Application-specific types are supported if they implement
/// [`Destroyable`]. The [`Guarded`] alias is provided for the common use-case where `Context`
/// is [`&ash::Device`](ash::Device).
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
pub struct GuardedResource<'a, Resource, Context>(
    // Invariant: The option is always Some, except possibly while being dropped.
    Option<ResourceAndContext<'a, Resource, Context>>,
)
where
    Resource: Destroyable,
    Context: Deref<Target = <Resource as Destroyable>::Context>;

#[derive(Debug)]
struct ResourceAndContext<'a, Resource, Context> {
    resource: Resource,
    context: Context,
    allocation_callbacks: Option<&'a vk::AllocationCallbacks>,
}

impl<'a, Resource, Context> GuardedResource<'a, Resource, Context>
where
    Resource: Destroyable,
    Context: Deref<Target = <Resource as Destroyable>::Context>,
{
    /// Creates a [`GuardedResource`] to hold the passed `resource`. `context` and
    /// `allocation_callbacks` are used during destruction.
    ///
    /// # Safety
    ///
    /// You must ensure that it is safe to destroy `resource` when the [`GuardedResource`] is
    /// dropped.
    pub unsafe fn new(
        resource: Resource,
        context: Context,
        allocation_callbacks: Option<&'a vk::AllocationCallbacks>,
    ) -> Self {
        Self(Some(ResourceAndContext {
            resource,
            context,
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

impl<'a, Resource, Context> AsRef<Resource> for GuardedResource<'a, Resource, Context>
where
    Resource: Destroyable,
    Context: Deref<Target = <Resource as Destroyable>::Context>,
{
    fn as_ref(&self) -> &Resource {
        &*self
    }
}

impl<'a, Resource, Context> AsMut<Resource> for GuardedResource<'a, Resource, Context>
where
    Resource: Destroyable,
    Context: Deref<Target = <Resource as Destroyable>::Context>,
{
    fn as_mut(&mut self) -> &mut Resource {
        &mut *self
    }
}

impl<'a, Resource, Context> Borrow<Resource> for GuardedResource<'a, Resource, Context>
where
    Resource: Destroyable,
    Context: Deref<Target = <Resource as Destroyable>::Context>,
{
    fn borrow(&self) -> &Resource {
        &*self
    }
}

impl<'a, Resource, Context> BorrowMut<Resource> for GuardedResource<'a, Resource, Context>
where
    Resource: Destroyable,
    Context: Deref<Target = <Resource as Destroyable>::Context>,
{
    fn borrow_mut(&mut self) -> &mut Resource {
        &mut *self
    }
}

impl<'a, Resource, Context> Deref for GuardedResource<'a, Resource, Context>
where
    Resource: Destroyable,
    Context: Deref<Target = <Resource as Destroyable>::Context>,
{
    type Target = Resource;

    fn deref(&self) -> &Self::Target {
        &self.0.as_ref().unwrap().resource
    }
}

impl<'a, Resource, Context> DerefMut for GuardedResource<'a, Resource, Context>
where
    Resource: Destroyable,
    Context: Deref<Target = <Resource as Destroyable>::Context>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.as_mut().unwrap().resource
    }
}

impl<'a, Resource, Context> Drop for GuardedResource<'a, Resource, Context>
where
    Resource: Destroyable,
    Context: Deref<Target = <Resource as Destroyable>::Context>,
{
    fn drop(&mut self) {
        if let Some(ResourceAndContext {
            resource,
            context,
            allocation_callbacks,
        }) = self.0.as_mut()
        {
            unsafe { resource.destroy_with(context, *allocation_callbacks) }
        }
    }
}
