use godot::prelude::*;
use godot::classes::RenderingDevice;
use super::SplatViewer;

pub fn dispatch_compute_raw(
    rd: &mut Gd<RenderingDevice>,
    pipeline_rid: Rid,
    uniform_set_rid: Rid,
    width: u32,
    height: u32,
) {
    let compute_list = rd.compute_list_begin();

    rd.compute_list_bind_compute_pipeline(compute_list, pipeline_rid);
    rd.compute_list_bind_uniform_set(compute_list, uniform_set_rid, 0);

    rd.compute_list_dispatch(
        compute_list,
        (width + 7) / 8,
        (height + 7) / 8,
        1,
    );

    rd.compute_list_end();
}

impl SplatViewer {
    pub fn dispatch_compute(&self, rd: &mut Gd<RenderingDevice>) {
        dispatch_compute_raw(
            rd,
            self.pipeline_rid,
            self.uniform_set_rid,
            self.width,
            self.height,
        );
    }
}
