use anyhow::Result;

pub fn run() -> Result<()> {
    let list = tauri_app_lib::get_ghost_list()?;

    println!("Built-in ghosts:");
    for ghost in &list.builtin {
        println!("  • {}", ghost);
    }

    println!("\nCustom ghosts:");
    if list.custom.is_empty() {
        println!("  (none yet - create with: medium ghosts scaffold <name>)");
    } else {
        for ghost in &list.custom {
            println!("  • {}", ghost);
        }
    }

    Ok(())
}
