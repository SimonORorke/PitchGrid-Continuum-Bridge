// As each test file is compiled separately, we have to add an `#[allow(dead_code)]` annotation
// to each module declaration to suppress the compiler warning there would otherwise be for each
// module that is not used in that particular test file.
#[allow(dead_code)] pub mod mock_continuum_protocol;
#[allow(dead_code)] pub mod mock_io;
#[allow(dead_code)] pub mod mock_midi_manager;
#[allow(dead_code)] pub mod mock_midi_sender;
#[allow(dead_code)] pub mod mock_osc;
#[allow(dead_code)] pub mod mock_release_info;
#[allow(dead_code)] pub mod mock_settings;
#[allow(dead_code)] pub mod mock_ui_methods;
#[allow(dead_code)] pub mod temp_path_finder;
#[allow(dead_code)] pub mod test_tunings;
