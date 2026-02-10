use std::sync::Arc;
// use std::sync::{Arc, Mutex};
use crate::osc::Osc;

pub struct Manager {
    osc: Osc,
    // osc: Arc<Mutex<Osc>>,
}

// impl Manager {
//     pub fn new() -> Self {
//         Manager {
//             osc: Osc::new(Arc::new(Self::on_osc_tuning_received), Arc::new(Self::on_osc_connected_changed))
//         }
//     }
//
//     fn on_osc_connected_changed(&mut self) {}
//
//     fn on_osc_tuning_received(&mut self, depth: i32, mode: i32, root_freq: f32, stretch: f32,
//                               skew: f32, mode_offset: i32, steps: i32) {
//
//     }
// }