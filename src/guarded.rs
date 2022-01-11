use std::borrow::{Borrow, BorrowMut};
use std::convert::{AsMut, AsRef};
use std::ops::{Deref, DerefMut};

use ash::vk;

use crate::Destroyable;

/// Most common usecase for [`GuardedResource`]
///
/// Fine-grained RAII should be short-lived, making references preferred.
///
/// Note that `'d` can be `'static` when `Resource` is [`ash::Instance`] or [`ash::Device`].
pub type Guarded<'d, Resource> =
    GuardedResource<'static, Resource, &'d <Resource as Destroyable>::Destroyer>;

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

    /// Returns the destroyer smartpointer/reference passed during construction.
    pub fn destroyer(&self) -> Destroyer
    where
        Destroyer: Clone,
    {
        self.0.as_ref().unwrap().destroyer.clone()
    }

    /// Returns the allocation callbacks passed during construction.
    pub fn allocation_callbacks(&self) -> Option<&'alloc_cb vk::AllocationCallbacks> {
        self.0.as_ref().unwrap().allocation_callbacks
    }

    /// Extracts the inner value without destroying it.
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

impl<'alloc_cb, Resource, Destroyer> GuardedResource<'alloc_cb, Vec<Resource>, Destroyer>
where
    Resource: Destroyable,
    Destroyer: Deref<Target = <Resource as Destroyable>::Destroyer>,
{
    /// Creates a [`GuardedResource`] to hold a [`Vec<Resource>`] populated from `resources`.
    /// `destroyer` and `allocation_callbacks` are used during destruction.
    ///
    /// If the iterator returns an error, then iteration is aborted and all resources created thus
    /// far are destroyed in first-to-last order.
    ///
    /// # Safety
    ///
    /// You must ensure that it is safe to destroy the resources when the [`GuardedResource`] is
    /// dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ash::{prelude::VkResult, vk};
    /// use ashpan::Guarded;
    ///
    /// unsafe fn create_swapchain_image_views(
    ///     device: &ash::Device,
    ///     swapchain_fn: ash::extensions::khr::Swapchain,
    ///     swapchain: vk::SwapchainKHR,
    ///     format: vk::Format,
    /// ) -> VkResult<Guarded<Vec<vk::ImageView>>> {
    ///     let subresource_range = vk::ImageSubresourceRange::builder()
    ///         .aspect_mask(vk::ImageAspectFlags::COLOR)
    ///         .level_count(1)
    ///         .layer_count(1)
    ///         .build();
    ///     let images = swapchain_fn.get_swapchain_images(swapchain)?;
    ///     let image_views = images.into_iter().map(|image| {
    ///         let create_info = vk::ImageViewCreateInfo::builder()
    ///             .view_type(vk::ImageViewType::TYPE_2D)
    ///             .format(format)
    ///             .subresource_range(subresource_range)
    ///             .image(image);
    ///         device.create_image_view(&create_info, None)
    ///     });
    ///     Guarded::try_new_from(image_views, device, None)
    /// }
    ///
    /// ```
    pub unsafe fn try_new_from<E>(
        resources: impl IntoIterator<Item = Result<Resource, E>>,
        destroyer: Destroyer,
        allocation_callbacks: Option<&'alloc_cb vk::AllocationCallbacks>,
    ) -> Result<Self, E> {
        // TODO: imitate Vec::extend_desugared()'s capacity management?
        let resources = resources.into_iter();
        let (min_capacity, _) = resources.size_hint();
        let mut guarded_resources = Self::new(
            Vec::with_capacity(min_capacity),
            destroyer,
            allocation_callbacks,
        );
        for resource in resources {
            guarded_resources.push(resource?);
        }
        Ok(guarded_resources)
    }
}

impl<'alloc_cb, Resource, Destroyer, const N: usize>
    GuardedResource<'alloc_cb, [Resource; N], Destroyer>
where
    Resource: Destroyable,
    Destroyer: Deref<Target = <Resource as Destroyable>::Destroyer>,
{
    /// Creates a [`GuardedResource`] to hold an array of resources. `destroyer` and
    /// `allocation_callbacks` are used during destruction.
    ///
    /// The array of resources is populated by repeatedly calling `resource_factory(index)`.
    /// If an error is encountered, resource creation is aborted and all resources created thus
    /// far are destroyed in first-to-last order.
    ///
    /// # Safety
    ///
    /// You must ensure that it is safe to destroy the resources when the [`GuardedResource`] is
    /// dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ash::{prelude::VkResult, vk};
    /// use ashpan::Guarded;
    ///
    /// unsafe fn create_frame_image_views<const N: usize>(
    ///     device: &ash::Device,
    ///     frame_images: vk::Image,
    ///     format: vk::Format,
    /// ) -> VkResult<Guarded<[vk::ImageView; N]>> {
    ///     let create_image_view = |layer| {
    ///         let subresource_range = vk::ImageSubresourceRange::builder()
    ///             .aspect_mask(vk::ImageAspectFlags::COLOR)
    ///             .level_count(1)
    ///             .base_array_layer(layer as u32)
    ///             .layer_count(1)
    ///             .build();
    ///         let create_info = vk::ImageViewCreateInfo::builder()
    ///             .view_type(vk::ImageViewType::TYPE_2D)
    ///             .format(format)
    ///             .subresource_range(subresource_range)
    ///             .image(frame_images);
    ///         device.create_image_view(&create_info, None)
    ///     };
    ///     Guarded::try_new_with(create_image_view, device, None)
    /// }
    ///
    /// ```
    pub unsafe fn try_new_with<E>(
        mut resource_factory: impl FnMut(usize) -> Result<Resource, E>,
        destroyer: Destroyer,
        allocation_callbacks: Option<&'alloc_cb vk::AllocationCallbacks>,
    ) -> Result<Self, E> {
        let mut resources = [(); N].map(|_| None);

        for (i, resource) in resources.iter_mut().enumerate() {
            *resource = Some(GuardedResource::new(
                resource_factory(i)?,
                &*destroyer,
                allocation_callbacks,
            ));
        }

        let resources = resources.map(|resource| {
            resource
                .expect("Bug in GuardedResource::new_with(): uninitialized resources")
                .take()
        });

        Ok(Self::new(resources, destroyer, allocation_callbacks))
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

#[cfg(test)]
mod tests {
    use crate::{Destroyable, Guarded, GuardedResource};
    use ash::vk;

    #[derive(Debug, PartialEq)]
    struct DestructorCalled<Destroyer> {
        destroyer: Destroyer,
        allocation_callbacks: Option<*const vk::AllocationCallbacks>,
    }

    #[derive(Debug)]
    struct TestResource<'a, Destroyer>(&'a mut Option<DestructorCalled<Destroyer>>);

    impl<'a, Destroyer: Copy> Destroyable for TestResource<'a, Destroyer> {
        type Destroyer = Destroyer;

        unsafe fn destroy_with(
            &mut self,
            &destroyer: &Destroyer,
            allocation_callbacks: Option<&vk::AllocationCallbacks>,
        ) {
            *(self.0) = Some(DestructorCalled {
                destroyer,
                allocation_callbacks: allocation_callbacks.map(|a| a as _),
            });
        }
    }

    #[derive(Debug)]
    struct TestWrapper<T>(T);

    impl<T: Clone> TestWrapper<T> {
        fn value(&self) -> T {
            self.0.clone()
        }

        fn set_value(&mut self, new_value: T) {
            self.0 = new_value;
        }
    }

    impl<T> Destroyable for TestWrapper<T> {
        type Destroyer = ();

        unsafe fn destroy_with(
            &mut self,
            _destroyer: &(),
            _allocation_callbacks: Option<&vk::AllocationCallbacks>,
        ) {
        }
    }

    #[test]
    fn methods_can_be_called_on_guarded_resource() {
        let mut guarded = unsafe { Guarded::new(TestWrapper(12332), &(), None) };
        assert_eq!(guarded.value(), 12332);
        guarded.set_value(42);
        assert_eq!(guarded.value(), 42);
    }

    #[test]
    fn guarded_resources_are_destroyed_when_dropped() {
        let allocation_callbacks = Default::default();
        let mut destructor_called = None;
        let resource = TestResource(&mut destructor_called);

        {
            let _guarded =
                unsafe { GuardedResource::new(resource, &(), Some(&allocation_callbacks)) };
        }

        assert_eq!(
            destructor_called,
            Some(DestructorCalled {
                destroyer: (),
                allocation_callbacks: Some(&allocation_callbacks as _)
            })
        );
    }

    #[test]
    fn guarded_resources_are_not_destroyed_when_taken() {
        let allocation_callbacks = Default::default();
        let mut destructor_called = None;
        let resource = TestResource(&mut destructor_called);

        {
            let guarded =
                unsafe { GuardedResource::new(resource, &(), Some(&allocation_callbacks)) };
            guarded.take();
        }

        assert!(destructor_called.is_none());
    }

    #[test]
    fn guarded_vec_has_accessible_elements() {
        let resources_to_create: [Result<_, ()>; 3] = [
            Ok(TestWrapper(321)),
            Ok(TestWrapper(432)),
            Ok(TestWrapper(543)),
        ];

        let guarded = unsafe { Guarded::try_new_from(resources_to_create, &(), None) }.unwrap();

        assert_eq!(guarded[0].value(), 321);
        assert_eq!(guarded[1].value(), 432);
        assert_eq!(guarded[2].value(), 543);
    }

    #[test]
    fn guarded_vec_destroys_elements_upon_drop() {
        let allocation_callbacks: vk::AllocationCallbacks = Default::default();
        let mut destructor_called_0 = None;
        let mut destructor_called_1 = None;
        let mut destructor_called_2 = None;

        {
            let resources_to_create: [Result<_, ()>; 3] = [
                Ok(TestResource(&mut destructor_called_0)),
                Ok(TestResource(&mut destructor_called_1)),
                Ok(TestResource(&mut destructor_called_2)),
            ];

            let mut guarded = unsafe {
                GuardedResource::try_new_from(resources_to_create, &42, Some(&allocation_callbacks))
            }
            .unwrap();

            guarded.pop();
        }

        assert_eq!(
            destructor_called_0,
            Some(DestructorCalled {
                destroyer: 42,
                allocation_callbacks: Some(&allocation_callbacks as _)
            })
        );
        assert_eq!(
            destructor_called_1,
            Some(DestructorCalled {
                destroyer: 42,
                allocation_callbacks: Some(&allocation_callbacks as _)
            })
        );
        assert!(destructor_called_2.is_none());
    }

    #[test]
    fn guarded_vec_drops_previously_created_elements_upon_error() {
        let allocation_callbacks: vk::AllocationCallbacks = Default::default();
        let mut destructor_called_0 = None;
        let mut destructor_called_1 = None;
        let mut destructor_called_2 = None;

        {
            let resources_to_create = [
                Ok(TestResource(&mut destructor_called_0)),
                Ok(TestResource(&mut destructor_called_1)),
                Err("oh no"),
                Err("another failure"),
                Ok(TestResource(&mut destructor_called_2)),
            ];

            let _guarded = unsafe {
                GuardedResource::try_new_from(resources_to_create, &42, Some(&allocation_callbacks))
            };
        }

        assert_eq!(
            destructor_called_0,
            Some(DestructorCalled {
                destroyer: 42,
                allocation_callbacks: Some(&allocation_callbacks as _)
            })
        );
        assert_eq!(
            destructor_called_1,
            Some(DestructorCalled {
                destroyer: 42,
                allocation_callbacks: Some(&allocation_callbacks as _)
            })
        );
        assert!(destructor_called_2.is_none());
    }

    #[test]
    fn guarded_vec_returns_first_error() {
        let resources_to_create = [
            Ok(TestWrapper(5)),
            Err("oh no"),
            Err("another failure"),
            Ok(TestWrapper(15)),
        ]
        .into_iter();

        let guarded = unsafe { GuardedResource::try_new_from(resources_to_create, &(), None) };

        assert_eq!(guarded.unwrap_err(), "oh no");
    }

    #[test]
    fn guarded_array_has_accessible_elements() {
        let mut values = [321, 432, 543].into_iter();
        let create_resource = |_| Result::<_, ()>::Ok(TestWrapper(values.next().unwrap()));

        let guarded = unsafe { Guarded::try_new_with(create_resource, &(), None) }.unwrap();

        assert_eq!(guarded[0].value(), 321);
        assert_eq!(guarded[1].value(), 432);
        assert_eq!(guarded[2].value(), 543);

        let _: [_; 3] = guarded.take();
    }

    #[test]
    fn guarded_array_destroys_elements_upon_drop() {
        let allocation_callbacks: vk::AllocationCallbacks = Default::default();
        let mut destructor_called_0 = None;
        let mut destructor_called_1 = None;
        let mut destructor_called_2 = None;

        {
            let resources_to_create: [Result<_, ()>; 3] = [
                Ok(TestResource(&mut destructor_called_0)),
                Ok(TestResource(&mut destructor_called_1)),
                Ok(TestResource(&mut destructor_called_2)),
            ];
            let mut resources_to_create = resources_to_create.into_iter();
            let create_resource = |_| resources_to_create.next().unwrap();

            let _guarded: GuardedResource<[_; 3], _> = unsafe {
                GuardedResource::try_new_with(create_resource, &42, Some(&allocation_callbacks))
            }
            .unwrap();
        }

        assert_eq!(
            destructor_called_0,
            Some(DestructorCalled {
                destroyer: 42,
                allocation_callbacks: Some(&allocation_callbacks as _)
            })
        );
        assert_eq!(
            destructor_called_1,
            Some(DestructorCalled {
                destroyer: 42,
                allocation_callbacks: Some(&allocation_callbacks as _)
            })
        );
        assert_eq!(
            destructor_called_2,
            Some(DestructorCalled {
                destroyer: 42,
                allocation_callbacks: Some(&allocation_callbacks as _)
            })
        );
    }

    #[test]
    fn guarded_array_drops_previously_created_elements_upon_error() {
        let allocation_callbacks: vk::AllocationCallbacks = Default::default();
        let mut destructor_called_0 = None;
        let mut destructor_called_1 = None;
        let mut destructor_called_2 = None;

        {
            let mut resources_to_create = [
                Ok(TestResource(&mut destructor_called_0)),
                Ok(TestResource(&mut destructor_called_1)),
                Err("oh no"),
                Ok(TestResource(&mut destructor_called_2)),
            ]
            .into_iter();
            let create_resource = |_| resources_to_create.next().unwrap();

            let _guarded: Result<GuardedResource<[_; 4], _>, _> = unsafe {
                GuardedResource::try_new_with(create_resource, &42, Some(&allocation_callbacks))
            };
        }

        assert_eq!(
            destructor_called_0,
            Some(DestructorCalled {
                destroyer: 42,
                allocation_callbacks: Some(&allocation_callbacks as _)
            })
        );
        assert_eq!(
            destructor_called_1,
            Some(DestructorCalled {
                destroyer: 42,
                allocation_callbacks: Some(&allocation_callbacks as _)
            })
        );
        assert!(destructor_called_2.is_none());
    }

    #[test]
    fn guarded_array_returns_first_error() {
        let mut resources_to_create = [
            Ok(TestWrapper(5)),
            Err("oh no"),
            Err("another failure"),
            Ok(TestWrapper(15)),
        ]
        .into_iter();
        let create_resource = |_| resources_to_create.next().unwrap();

        let guarded: Result<GuardedResource<[_; 4], _>, _> =
            unsafe { GuardedResource::try_new_with(create_resource, &(), None) };

        assert_eq!(guarded.unwrap_err(), "oh no");
    }
}
