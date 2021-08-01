# Ashpan

[![Crates.io Version](https://img.shields.io/crates/v/ashpan.svg)](https://crates.io/crates/ashpan)
[![Documentation](https://docs.rs/ashpan/badge.svg)](https://docs.rs/ashpan)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE-APACHE)

RAII helpers for ash

[`ashpan`](https://crates.io/crates/ashpan) makes it easier to properly clean
up [`ash`](https://crates.io/crates/ash) resources when failures occur. It's
essentially a [`scopeguard`](https://crates.io/crates/scopeguard) that has been
tailored to Vulkan.

## Example

```rust
use ashpan::{DeviceExt, Guarded};

struct Resources { ... }

unsafe fn create_resources(device: &ash::Device) -> VkResult<Resources> {
    let render_pass = create_render_pass(device)?;
    let pipeline_layout = create_pipeline_layout(device)?;
    let pipeline = create_pipeline(device, *render_pass, *pipeline_layout)?;
    Ok(Resources {
        render_pass: render_pass.take(),
        pipeline_layout: pipeline_layout.take(),
        pipeline: pipeline.take(),
    })
}

unsafe fn create_render_pass(device: &ash::Device) -> VkResult<Guarded<vk::RenderPass>> {
    let create_info = unimplemented!();
    device.create_guarded_render_pass(create_info, None)
}
```

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
