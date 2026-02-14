use std::{fs, path::Path};

use include_dir::Dir;

pub fn extract_template(template: &Dir, output_path: &str) -> std::io::Result<()> {
    fs::create_dir_all(output_path)?;

    extract_dir_contents(template, Path::new(output_path))?;

    Ok(())
}

pub fn extract_dir_contents(dir: &Dir, base_path: &Path) -> std::io::Result<()> {
    for file in dir.files() {
        let file_name = file.path().file_name().unwrap();
        let file_path = base_path.join(file_name);
        fs::write(&file_path, file.contents())?;
        println!("Created: {}", file_path.display());
    }

    for subdir in dir.dirs() {
        let subdir_name = subdir.path().file_name().unwrap();
        let subdir_path = base_path.join(subdir_name);

        fs::create_dir_all(&subdir_path)?;
        extract_dir_contents(subdir, &subdir_path)?;
    }

    Ok(())
}
