use std::env;
use std::fs;
use std::io::{self, Read};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut ast_mode = false;
    let mut file_path: Option<String> = None;

    for arg in args.iter().skip(1) {
        if arg == "--ast" {
            ast_mode = true;
        } else {
            file_path = Some(arg.clone());
        }
    }

    let input = match file_path {
        Some(path) => fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("Failed to read {}: {}", path, e);
            std::process::exit(1);
        }),
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf).unwrap_or_else(|e| {
                eprintln!("Failed to read stdin: {}", e);
                std::process::exit(1);
            });
            buf
        }
    };

    if ast_mode {
        let doc = rfm::parse(&input);
        println!("{:#?}", doc);
    } else {
        let html = rfm::to_html(&input);
        print!("{}", html);
    }
}
