#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]

use anyhow::*;
use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;

fn main() -> Result<()> {
    // This tells cargo to rerun this script if something in /res/ changes.
    println!("cargo:rerun-if-changed=./res/*");

    // Copy the /res/ folder to the output directory.
    let mut copyOptions = CopyOptions::new();
    copyOptions.overwrite = true;
    let pathsToCopy = vec!["../res/"];

    // Make sure the directories exist.
    std::fs::create_dir_all("./../target-examples/debug/")?;
    std::fs::create_dir_all("./../target-examples/release/")?;

    // Check if we're in debug or release mode.
    if cfg!(debug_assertions) {
        // Copy the /res/ folder to the output directory.
        copy_items(&pathsToCopy, "./../target-examples/debug/", &copyOptions)?;
    } else {
        // Copy the /res/ folder to the output directory.
        copy_items(&pathsToCopy, "./../target-examples/release/", &copyOptions)?;
    }

    Ok(())
}
