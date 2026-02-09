// const HOST: &str = "127.0.0.1";
// const SERVER_PORT: u16 = 34561;
// const CLIENT_PORT: u16 = 34562;
//
// struct Osc {
//     tuning_callback: Box<dyn Fn(
//         i32, // depth
//         i32, // mode
//         f32, // root_freq
//         i32, // stretch
//         f32, // skew
//         i32, // mode_offset
//         i32)>, // steps
// }
//
// impl Osc {
//     pub fn new(tuning_callback: Box<dyn Fn(i32, i32, f32, i32, f32, i32, i32)>) -> Self {
//         Self { tuning_callback }
//     }
//
//     pub fn start(&self) {}
// }