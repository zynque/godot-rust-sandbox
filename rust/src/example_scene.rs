//! Builds the growth demo scene and hands it to a `ParcelRendererNode`.

use godot::prelude::*;

use growth::example_scene::example_scene_parcels_at;
use growth::parcels::PARCEL_FLOATS;

#[derive(GodotClass)]
#[class(tool, base=Node)]
struct ExampleSceneNode {
    base: Base<Node>,
    #[export]
    renderer_path: NodePath,
    renderer: Option<Gd<Node>>,
    elapsed_seconds: f32,
    animate_lookup_failed: bool,
}

#[godot_api]
impl INode for ExampleSceneNode {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            renderer_path: NodePath::default(),
            renderer: None,
            elapsed_seconds: 0.0,
            animate_lookup_failed: false,
        }
    }

    fn ready(&mut self) {
        self.base_mut().set_process(true);

        if self.renderer_path.get_name_count() == 0 {
            godot_error!("ExampleSceneNode: renderer_path is not set; no parcels were sent.");
            return;
        }

        let Some(mut renderer) = self.base().try_get_node_as::<Node>(&self.renderer_path) else {
            godot_error!(
                "ExampleSceneNode: renderer_path '{}' does not point at a node.",
                self.renderer_path,
            );
            return;
        };

        if !renderer.has_method("set_parcels") {
            godot_error!(
                "ExampleSceneNode: node at '{}' has no set_parcels method.",
                self.renderer_path,
            );
            return;
        }

        let count = self.send_parcels(&mut renderer, 0.0);
        godot_print!(
            "ExampleSceneNode: sending {} parcels to '{}'.",
            count,
            self.renderer_path,
        );
        self.renderer = Some(renderer);
    }

    fn process(&mut self, delta: f64) {
        let Some(mut renderer) = self.renderer.clone() else {
            return;
        };
        if !renderer.is_instance_valid() {
            self.renderer = None;
            return;
        }

        // The renderer's animate flag drives the scene time, so the parcels
        // move exactly when its shader time advances.
        let animate = match renderer.get("animate").try_to::<bool>() {
            Ok(animate) => animate,
            Err(error) => {
                if !self.animate_lookup_failed {
                    godot_error!(
                        "ExampleSceneNode: node at '{}' has no readable bool 'animate' property: {}",
                        self.renderer_path,
                        error,
                    );
                    self.animate_lookup_failed = true;
                }
                return;
            }
        };

        if !animate {
            return;
        }

        self.elapsed_seconds += delta as f32;
        self.send_parcels(&mut renderer, self.elapsed_seconds);
    }
}

impl ExampleSceneNode {
    fn send_parcels(&self, renderer: &mut Gd<Node>, time_seconds: f32) -> usize {
        let parcels = example_scene_parcels_at(time_seconds);
        let mut packed = Vec::with_capacity(parcels.len() * PARCEL_FLOATS);
        for parcel in &parcels {
            packed.extend_from_slice(&parcel.as_f32s());
        }

        renderer.call(
            "set_parcels",
            &[PackedFloat32Array::from(packed.as_slice()).to_variant()],
        );

        parcels.len()
    }
}
