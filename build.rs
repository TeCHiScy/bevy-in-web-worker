use dom_query::Document;
use std::fs::File;
use std::io::Write;

fn parse(text: &str) -> Vec<Entry> {
    let doc = Document::from(text);
    let mut display = Vec::new();

    for table in doc.select("table").iter() {
        let table_id = table.attr_or("id", "").to_string();

        // Skip duplicate table
        if table_id == "key-table-media-controller-dup" {
            continue;
        }

        // Mark legacy modifier keys as deprecated
        let deprecated =
            table_id == "key-table-modifier-legacy" || table_id == "table-key-code-legacy-modifier";

        for row in table.select("tbody tr").iter() {
            let cells: Vec<_> = row.select("td").nodes().to_vec();
            if cells.len() < 3 {
                continue;
            }

            let name = cells[0].text().trim().trim_matches('"').to_string();

            // Skip F keys here
            if name.starts_with('F') && name[1..].parse::<u32>().is_ok() {
                continue;
            }

            //  Strip <a> tags
            for a in row.select("td:nth-child(3) > a").iter() {
                a.replace_with_html(a.text());
            }

            // Use the semantic `<kbd>` element instead.
            for kbd in row.select("td:nth-child(3) > code.keycap").iter() {
                kbd.replace_with_html(format!("<kbd>{}</kbd>", kbd.text()));
            }

            // Link to the relevant type.
            for code in row.select("td:nth-child(3) > code.code").iter() {
                let text = code.text().trim_matches('"').to_string();
                code.replace_with_html(format!("[`{text}`][Code::{text}]"));
            }

            for key in row.select("td:nth-child(3) > code.key").iter() {
                let text = key.text().trim_matches('"').to_string();
                key.replace_with_html(format!("[`{text}`][NamedKey::{text}]"));
            }

            // Strip <a> tags - replace with text content
            let typical_usage = cells[2]
                .inner_html()
                .replace("\t", "\n")
                .replace("<br>", "\n")
                .lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join("\n")
                .to_string();

            display.push((name, typical_usage, deprecated, Vec::new(), Vec::new()));
        }
    }

    display
}

fn print_keys_codes(keys: &[Entry], codes: &[Entry], file: &mut File) -> std::io::Result<()> {
    // https://github.com/rust-windowing/winit/blob/da6220060e7626c11332354cc26cd47e2937c200/winit-web/src/web_sys/event.rs#L257
    // https://github.com/bevyengine/bevy/blob/24729499ed63606d99969a78792b4246fa139dd4/crates/bevy_winit/src/converters.rs#L305
    // https://github.com/bevyengine/bevy/blob/24729499ed63606d99969a78792b4246fa139dd4/crates/bevy_winit/src/converters.rs#L634
    // Key::Unidentified(NativeKey::Web(SmolStr::new("Unidentified")))
    // Key::Unidentified(nk) => bevy_input::keyboard::Key::Unidentified(convert_native_key(nk)),
    // NativeKey::Web(v) => bevy_input::keyboard::NativeKey::Web(v.clone()),

    // https://github.com/rust-windowing/keyboard-types/blob/b2c511acd8e54224cec5535bfb96d48cd4c75e3f/src/named_key.rs#L1067
    // https://github.com/rust-windowing/winit/blob/da6220060e7626c11332354cc26cd47e2937c200/winit-web/src/web_sys/event.rs#L259
    // https://github.com/bevyengine/bevy/blob/24729499ed63606d99969a78792b4246fa139dd4/crates/bevy_winit/src/converters.rs#L306
    // "Dead" => Ok(Dead),
    // Ok(NamedKey::Dead) => Key::Dead(None),
    // Key::Dead(c) => bevy_input::keyboard::Key::Dead(c.to_owned()),

    // https://github.com/rust-windowing/keyboard-types/blob/b2c511acd8e54224cec5535bfb96d48cd4c75e3f/src/named_key.rs#L1308
    // https://github.com/rust-windowing/winit/blob/da6220060e7626c11332354cc26cd47e2937c200/winit-web/src/web_sys/event.rs#L261
    // https://github.com/bevyengine/bevy/blob/24729499ed63606d99969a78792b4246fa139dd4/crates/bevy_winit/src/converters.rs#L304
    // _ => Err(UnrecognizedNamedKeyError),
    // Err(_) => Key::Character(SmolStr::new(key)),
    // Key::Character(s) => bevy_input::keyboard::Key::Character(s.clone()),
    for (key, _, _, alternatives, _) in keys {
        write!(file, "            \"{}\"", key)?;
        for alternative in alternatives {
            write!(file, " | \"{}\"", alternative)?;
        }

        if key == "Dead" {
            writeln!(file, " => Key::Dead(None),")?;
        } else if key == "Unidentified" {
            writeln!(
                file,
                " => Key::Unidentified(NativeKey::Web(SmolStr::new(\"Unidentified\"))),"
            )?;
        } else {
            writeln!(file, " => Key::{},", key)?;
        }
    }

    for (key, _, _, alternatives, _) in codes {
        if keys.iter().any(|(k, _, _, _, _)| k == key) {
            continue;
        }
        write!(file, "            \"{}\"", key)?;
        for alternative in alternatives {
            write!(file, " | \"{}\"", alternative)?;
        }
        writeln!(file, " => Key::Character(SmolStr::new(\"{}\")),", key)?;
    }
    Ok(())
}

fn print_codes(codes: &[Entry], file: &mut File) -> std::io::Result<()> {
    for (key, _, _, alternatives, _) in codes {
        if key == "Unidentified" {
            continue;
        }
        let mut key = key.to_string();
        if key == "MetaLeft" || key == "MetaRight" || key == "Super" {
            key = "Meta".to_string();
        }
        write!(file, "            \"{}\"", key)?;
        for alternative in alternatives {
            write!(file, " | \"{}\"", alternative)?;
        }
        writeln!(file, " => KeyCode::{},", key)?;
    }
    Ok(())
}

fn add_comment_to(display: &mut [Entry], key: &str, comment: &str) {
    for (found_key, doc_comment, ..) in display.iter_mut() {
        if found_key == key {
            doc_comment.push('\n');
            doc_comment.push_str(comment);
        }
    }
}

fn add_alias_for(
    display: &mut [(String, String, bool, Vec<String>, Vec<String>)],
    key: &str,
    alias: &str,
) {
    for (found_key, _, _, _, aliases) in display.iter_mut() {
        if found_key == key {
            aliases.push(alias.to_string());
        }
    }
}

fn add_alternative_for(display: &mut [Entry], key: &str, alternative: &str) {
    for (found_key, _, _, alternatives, aliases) in display.iter_mut() {
        if found_key == key {
            alternatives.push(alternative.to_string());
            aliases.push(alternative.to_string());
        }
    }
}

fn keys(text: &str) -> Vec<Entry> {
    let mut entries = parse(text);
    for i in 1..36 {
        entries.push((
            format!("F{}", i),
            format!(
                "The F{0} key, a general purpose function key, as index {0}.",
                i
            ),
            false,
            Vec::new(),
            Vec::new(),
        ));
    }

    add_comment_to(
        &mut entries,
        "Meta",
        "In Linux (XKB) terminology, this is often referred to as \"Super\".",
    );
    add_alias_for(&mut entries, "Meta", "Super");
    add_alias_for(&mut entries, "Enter", "Return");
    entries
}

type Entry = (String, String, bool, Vec<String>, Vec<String>);

fn convert_key(keys: &[Entry], codes: &[Entry], mut file: &mut File) -> std::io::Result<()> {
    write!(
        file,
        r#"

pub trait AsKey {{
    fn as_key(&self) -> Key;
}}

impl AsKey for &str {{
    fn as_key(&self) -> Key {{
        match *self {{
"#
    )?;

    print_keys_codes(&keys, codes, &mut file)?;
    write!(
        file,
        r#"
            _ => todo!(),
        }}
    }}
}}"#,
    )?;

    Ok(())
}

fn codes(text: &str) -> Vec<Entry> {
    let mut display = parse(text);
    for i in 1..36 {
        display.push((
            format!("F{}", i),
            format!("<kbd>F{}</kbd>", i),
            false,
            Vec::new(),
            Vec::new(),
        ));
    }

    add_comment_to(
        &mut display,
        "Backquote",
        "This is also called a backtick or grave.",
    );
    add_comment_to(&mut display, "Quote", "This is also called an apostrophe.");
    add_comment_to(
        &mut display,
        "MetaLeft",
        "In Linux (XKB) terminology, this is often referred to as the left \"Super\".",
    );
    add_comment_to(
        &mut display,
        "MetaRight",
        "In Linux (XKB) terminology, this is often referred to as the right \"Super\".",
    );

    add_alias_for(&mut display, "Backquote", "Backtick");
    add_alias_for(&mut display, "Backquote", "Grave");
    add_alias_for(&mut display, "Quote", "Apostrophe");
    add_alias_for(&mut display, "MetaLeft", "SuperLeft");
    add_alias_for(&mut display, "MetaRight", "SuperRight");
    add_alias_for(&mut display, "Enter", "Return");

    add_alternative_for(&mut display, "MetaLeft", "OSLeft");
    add_alternative_for(&mut display, "MetaRight", "OSRight");
    add_alternative_for(&mut display, "AudioVolumeDown", "VolumeDown");
    add_alternative_for(&mut display, "AudioVolumeMute", "VolumeMute");
    add_alternative_for(&mut display, "AudioVolumeUp", "VolumeUp");
    add_alternative_for(&mut display, "MediaSelect", "LaunchMediaPlayer");

    display
}

fn header(file: &mut File) -> std::io::Result<()> {
    write!(
        file,
        r#"// AUTO GENERATED CODE - DO NOT EDIT
#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::doc_markdown)]
#![allow(deprecated)]

use bevy_input::keyboard::{{NativeKey, Key, NativeKeyCode, KeyCode}};
use smol_str::SmolStr;"#
    )?;

    Ok(())
}

fn convert_code(codes: &[Entry], mut file: &mut File) -> std::io::Result<()> {
    write!(
        file,
        r#" 
pub trait AsKeyCode {{
    fn as_key_code(&self) -> KeyCode;
}}

impl AsKeyCode for &str {{
    fn as_key_code(&self) -> KeyCode {{
        match *self {{
"#
    )?;
    print_codes(&codes, &mut file)?;
    write!(
        file,
        r#"
            _ => KeyCode::Unidentified(NativeKeyCode::Unidentified),
        }}
    }}
}}"#
    )?;

    Ok(())
}

// https://github.com/rust-windowing/winit/blob/master/winit-web/src/web_sys/event.rs
// https://github.com/rust-windowing/keyboard-types/blob/main/convert.py
// https://github.com/rust-windowing/keyboard-types/blob/main/src/code.rs#L716
// https://github.com/rust-windowing/keyboard-types/blob/main/src/named_key.rs
// Use keyboard-types' parsing (it is based on the W3C standard).
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create("src/keyboard.rs")?;
    let key = reqwest::blocking::get("https://w3c.github.io/uievents-key/")?.text()?;
    let keys = keys(&key);
    let code = reqwest::blocking::get("https://w3c.github.io/uievents-code/")?.text()?;
    let codes = codes(&code);
    header(&mut file)?;
    convert_key(&keys, &codes, &mut file)?;
    convert_code(&codes, &mut file)?;
    Ok(())
}
