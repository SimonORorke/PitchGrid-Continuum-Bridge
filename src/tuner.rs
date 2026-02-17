#[cxx::bridge(namespace = "sx")]
mod ffi {
    unsafe extern "C++" {
        include!("scalatrix.hpp");
        // type MOS;
        // fn new_MOS(depth: i32, mode: i32, root_freq: f32, stretch: f32,
        //            skew: f32, mode_offset: i32, steps: i32) -> UniquePtr<MOS>;
        //
    }
}

#[allow(unused)]
pub fn on_tuning_changed(depth: i32, mode: i32, root_freq: f32, stretch: f32,
                          skew: f32, mode_offset: i32, steps: i32) {
    // let mos = unsafe { ffi::new_MOS(depth, mode, skew as f64, stretch as f64, 1) };
}
