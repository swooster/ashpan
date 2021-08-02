# Version 0.3.0

* Removed `scopeguard` dependency. (technically a breaking change because
  `DeviceExt` now requires `Deref<Target=ash::Device>`)

# Version 0.2.0

## Breaking changes

* Renamed `EntryGuardedMethods` to `EntryExt`
* Renamed `InstanceGuardedMethods` to `InstanceExt`
* Renamed `DeviceGuardedMethods` to `DeviceExt`

# Version 0.1.0

First release!
