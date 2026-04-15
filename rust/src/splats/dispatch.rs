use godot::prelude::*;
use godot::classes::RenderingDevice;
use super::SplatViewer;

impl SplatViewer {
  pub fn dispatch_compute(&self, rd: &mut Gd<RenderingDevice>) {
        let compute_list = rd.compute_list_begin();

        rd.compute_list_bind_compute_pipeline(compute_list, self.pipeline_rid);
        rd.compute_list_bind_uniform_set(compute_list, self.uniform_set_rid, 0);

        rd.compute_list_dispatch(
            compute_list,
            (self.width / 8) as u32,
            (self.height / 8) as u32,
            1,
        );

        rd.compute_list_end();
        rd.submit();

        // For debugging only — remove later
        rd.sync();
    } 
}
