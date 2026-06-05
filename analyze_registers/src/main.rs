use ignore::{DirEntry, WalkBuilder, types::TypesBuilder};
use prettyplease::unparse;
use std::{fs::read_to_string, path::PathBuf};
use syn::{
    Fields, File, Ident,
    Item::{self, Macro, Mod, Struct},
    Meta::List,
    Token, parse_file,
    punctuated::Punctuated,
};

fn find_defs(items: Vec<Item>) {
    for item in items {
        match item {
            Macro(_item) => {} // TODO
            Mod(item) => {
                if let Some((_, content)) = item.content {
                    find_defs(content);
                }
            }
            Struct(item) => {
                if !item.attrs.iter().any(|a| a.path().is_ident("repr")) {
                    continue;
                };
                // TODO(CHECKPOINT): I'm still working out how to identify register structs. I was
                // hoping that only structs with named fields would be registers, but I found
                // ChannelPriorityRegisters which is a tuple struct with registers inside. I'm not
                // really sure where this tool should sit between a data-analysis tool and an
                // automatic-porting tool, so it's not clear if it really matters.
                match item.fields {
                    Fields::Named(_) => continue,
                    _ => {}
                }
                println!(
                    "{}",
                    unparse(&File {
                        shebang: None,
                        attrs: Vec::new(),
                        items: vec![Struct(item)],
                    })
                );
            }
            _ => {}
        }
    }
}

fn process_file(path: PathBuf) {
    let Ok(file) = parse_file(&read_to_string(&path).unwrap()) else {
        return;
    };
    //println!("{}:", path.display());
    find_defs(file.items);
    //println!();
}

fn main() {
    WalkBuilder::new("../../tock")
        .types(
            TypesBuilder::new()
                .add_defaults()
                .select("rust")
                .build()
                .unwrap(),
        )
        .build()
        .map(Result::unwrap)
        .filter(|d| d.file_type().map_or(false, |t| t.is_file()))
        .map(DirEntry::into_path)
        .for_each(process_file);
}
