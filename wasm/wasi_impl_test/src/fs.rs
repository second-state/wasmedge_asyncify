use std::fs;

pub fn list_cwd() {
    let dirs = fs::read_dir(".").unwrap();
    for dir in dirs {
        log::info!("{:?}", dir);
    }
}
