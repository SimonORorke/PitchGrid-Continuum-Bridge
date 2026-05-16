mod temp_path_finder;

use googletest::assert_that;
use googletest::matchers::{eq, ok};
use pitchgrid_continuum::settings::Settings;
use temp_path_finder::TempPathFinder;

#[googletest::gtest]
fn persist() {
    const PITCH_TABLE: u8 = 81;
    let mut settings = Settings::new();
    let temp_path_finder = TempPathFinder::new();
    settings.set_system_path_finder(Box::new(temp_path_finder.clone()));
    settings.pitch_table = PITCH_TABLE;
    assert_that!(settings.write_to_file(), ok(()));
    settings = Settings::new();
    settings.set_system_path_finder(Box::new(temp_path_finder.clone()));
    assert_that!(settings.read_from_file(), ok(()));
    assert_that!(settings.pitch_table, eq(PITCH_TABLE));
}
