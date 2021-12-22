# Releases

## Unreleased

* Update `ash` dependency to 0.34.0

## Version 0.5.2

* `GuardedResource` documentation fixes

## Version 0.5.1

* Added `destroyer` and `allocation_callbacks` accessors to `GuardedResource`.

## Version 0.5.0

* Removed `GuardedInstance` and `GuardedDevice` because `Guarded` now supports
  non-`ash::Device` destroyers; use `Guarded<'static, ash::Instance>` or
  `Guarded<'static, ash::Device>` instead.

## Version 0.4.3

* Give lifetimes in `Guarded` and `GuardedResource` (slightly) more descriptive
  abbreviated names rather just `'a`.
* Extended `Guarded` to work with all `Destroyable` resources, not just ones
  destroyable by `ash::Device`.

## Version 0.4.2

* Added `GuardedDevice` and `GuardedInstance` type aliases.

## Version 0.4.1

* Improved wording of `Destroyable` docs.

## Version 0.4.0

### Breaking changes

* Renamed `Destroyable::Context` to `Destroyable::Destroyer` to make the most
  common use-case more obvious.

## Version 0.3.0

* Removed `scopeguard` dependency. (technically a breaking change because
  `DeviceExt` now requires `Deref<Target=ash::Device>`)

## Version 0.2.0

### Breaking changes

* Renamed `EntryGuardedMethods` to `EntryExt`
* Renamed `InstanceGuardedMethods` to `InstanceExt`
* Renamed `DeviceGuardedMethods` to `DeviceExt`

## Version 0.1.0

First release!
