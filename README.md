# `bevy_iced`: use [iced](https://github.com/iced-rs/iced) UI programs in your [Bevy](https://github.com/bevyengine/bevy/) application

## Example

```rust
use bevy::prelude::*;
use bevy_iced::{
    IcedAppExtensions, IcedPlugin,
    iced::{Program, program::State}
};

#[derive(Default)]
pub struct Ui {
    // Set up your UI state
}

impl Program for Ui {
    // Set up your program logic
}

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(IcedPlugin)
        .insert_program(Ui::default())
        .add_system(ui_system)
        .run();
}

pub fn ui_system(mut ui_state: NonSendMut<State<Ui>>, /* ... */) {
    // Do some work here, then modify your ui state by running
    // ui_state.queue_message(..);
}
```

See the [examples](https://github.com/tasgon/bevy_iced/tree/master/examples) folder for more details on how to use the crate.

## Todo

- Add support for programs that are `Send + Sync` (It's actually pretty simple to support it, but I'd like to do it in a somewhat clean manner)
- Multi-window support
- Bind programs to individual stages

## Credits

- [`bevy_egui`](https://github.com/mvlabat/bevy_egui) for giving me a useful starting point to do this