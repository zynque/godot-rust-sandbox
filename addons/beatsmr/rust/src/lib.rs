use godot::prelude::*;

mod build_interval_cluster_tester;
mod count_overlapping_intervals_tester;
mod intersect_parcels_tester;
mod make_interval_buckets_tester;
mod parcel_renderer_node;
mod parcel_test_common;
mod shader_test_common;
mod sort_interval_cluster_tester;
mod sort_intervals_tester;
mod update_interval_bucket_weights_tester;

struct BeatsmrExtension;

#[gdextension]
unsafe impl ExtensionLibrary for BeatsmrExtension {}
