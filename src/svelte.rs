// Di file mod.rs atau svelte.rs
use axum::response::{Html, IntoResponse, Response};
use std::str;

pub async fn generate_svelte_template(headers: &str, data: &str, path: &str) -> String {
    // Ambil konten dari Asset yang sudah diembed
    
    // Ambil konten dari RustEmbed Assets
    if let Some(content) = super::Assets::get(path) {
        if headers == "navigate" {
            if let Ok(html_str) = str::from_utf8(&content.data) {
                let mut html_content = html_str.to_string();
                
                let script_start = html_content.find("<script>");
                if let Some(body_pos) = script_start {
                    let embed_str = format!(r#"let data2 = "{}";"#, data);
                    if let Some(data2_pos) = html_content.find("let data2 =") {
                        let end_pos = html_content[data2_pos..].find(';').unwrap() + data2_pos + 1;
                        html_content.replace_range(data2_pos..end_pos, &embed_str);
                    } else {
                        html_content.insert_str(body_pos + 8, &embed_str);
                    }
                    return html_content;
                }
            }
        }
    }
    "".to_string()
}