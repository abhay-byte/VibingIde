use std::fs;
fn main() {
    let mut c = fs::read_to_string("src/app.rs").unwrap();
    // Use highly flexible find to delete the Inter and Space Grotesk loads 
    // and the proportional family setup
    let start_str = "        fonts.font_data.insert(\n            \"Inter\".to_owned()";
    let end_str = "        fonts\n            .families\n            .entry(egui::FontFamily::Monospace)";
    
    // Support varying line endings (\n vs \r\n)
    let c_normalized = c.replace("\r\n", "\n");
    if let Some(start) = c_normalized.find(start_str) {
        if let Some(end) = c_normalized.find(end_str) {
            let to_remove = &c_normalized[start..end];
            c = c.replace(to_remove, "");
            fs::write("src/app.rs", c).unwrap();
            println!("Fix applied!");
        } else {
            println!("End not found");
        }
    } else {
        println!("Start not found");
    }
}
