# `bevy_iced`: use [Iced](https://github.com/iced-rs/iced) UI programs in your [Bevy](https://github.com/bevyengine/bevy/) application

[![Crates.io](https://img.shields.io/crates/v/bevy_iced.svg)](https://crates.io/crates/bevy_iced)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](./LICENSE)

## Example

```rust
use bevy::prelude::*;
use bevy_iced::iced::widget::text;
use bevy_iced::{IcedContext, IcedPlugin};

#[derive(Event)]
pub enum UiMessage {}

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(IcedPlugin::default())
        .add_event::<UiMessage>()
        .add_systems(Update, ui_system)
        .run();
}

fn ui_system(time: Res<Time>, mut ctx: IcedContext<UiMessage>) {
    ctx.display(text(format!(
        "Hello Iced! Running for {:.2} seconds.",
        time.elapsed_seconds()
    )));
}
```

See the [examples](https://github.com/tasgon/bevy_iced/tree/master/examples) and the [documentation](https://docs.rs/bevy_iced) for more details on how to use the crate.

## Compatibility

|Bevy Version  |Crate Version  |
|--------------|---------------|
|`0.13`        |`0.5`, `master`|
|`0.11`        |`0.4`          |
|`0.10`        |`0.3`          |
|`0.9`         |`0.2`          |
|`0.7`         |`0.1`          |

## Todo

- Multi-window support
- Clipboard support

## Credits

- [`bevy_egui`](https://github.com/mvlabat/bevy_egui) for giving me a useful starting point to do this
- [Joonas Satka](https://github.com/jsatka) for helping me port to Bevy 0.11
- [Tomas Zemanovic](https://github.com/tzemanovic) and [Julia Naomi](https://github.com/naomijub) for helping me port to Bevy 0.13