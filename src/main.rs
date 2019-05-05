#[macro_use]
extern crate diesel;

use std::env;
use std::error::Error;
use std::fs::File;
use std::hash::Hasher;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::exit;
use std::sync::mpsc::channel;
use std::time::Duration;

use diesel::{connection::Connection, SqliteConnection};
use diesel::prelude::*;
use dotenv::dotenv;
use notify;
use notify::{DebouncedEvent::*, RecommendedWatcher, RecursiveMode, Watcher};
use walkdir::WalkDir;
use yaml_rust::{Yaml, YamlLoader};

use schema::*;

mod schema;

static EXPLORER_WATCH_PATH: &'static str = "EXPLORER_WATCH_PATH";
static CARGO_MANIFEST_DIR: &'static str = "CARGO_MANIFEST_DIR";

fn main() {
    println!("Hello, world!");
    watch_events();
}

fn directory() -> PathBuf {
    match env::var(EXPLORER_WATCH_PATH) {
        Ok(val) => PathBuf::from(val),
        Err(_) => PathBuf::from(env::var(CARGO_MANIFEST_DIR).unwrap())
    }
}

fn startup(path: PathBuf, conn: &SqliteConnection) -> Result<(), Box<Error>> {
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if let Some(x) = entry.path().extension() {
            if x == "md" {
                let f = File::open(entry.path())?;
                let buf = BufReader::new(f);
                let mut content = String::new();
                for line in buf.lines() {
                    let l = line?;
                    if &l == "---" {
                        break;
                    }
                    content.push_str(&format!("{}\n", &l));
                }
                let ctnt = content.clone();
                let mut note: NoteMtda = content.into();
                if note.id == 0 {
                    use std::collections::hash_map::DefaultHasher;
                    let mut hasher = DefaultHasher::new();
                    hasher.write(&mut ctnt.as_bytes());
                    let val = hasher.finish();
                    note.id = val as i64;
                }
                use crate::schema::notes::dsl::*;
                use crate::schema::tags::dsl::*;
                diesel::insert_or_ignore_into(notes).values(Note {
                    id: note.id,
                    title: entry.file_name().to_string_lossy().into_owned(),
                }).execute(conn).expect("Error saving note");
                dbg!(&note.tags);
                note.tags.iter().for_each(|x| {
                    diesel::insert_or_ignore_into(tags).values(Tag {
                        noteId: note.id,
                        tag: x.to_string(),
                    }).execute(conn).expect("Error adding tag");
                });
            }
        }
    }
    Ok(())
}

fn establish_connection() -> SqliteConnection {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

fn watch_events() {
    let path = directory();
    let conn = establish_connection();
    startup(path.clone(), &conn).unwrap();
    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = match Watcher::new(tx, Duration::from_secs(2)) {
        Ok(d) => d,
        Err(_) => {
            println!("Provided path is invalid");
            exit(1);
        }
    };
    watcher.watch(path, RecursiveMode::Recursive).unwrap();
    loop {
        match rx.recv() {
            Ok(event) => {
                match event {
                    Create(e) => println!("{:#?}", e),
                    Write(e) => println!("{:#?}", e),
                    Remove(e) => println!("{:#?}", e),
                    Rename(e, o) => println!("{:#?} to {:#?}", e, o),
                    _ => println!("doing other stuff")
                }
            }
            Err(_) => {
                dbg!("Error");
            }
        }
    }
}

#[derive(Debug, Default)] //Queryable, Insertable, Identifiable, AsChangeset)]
//#[table_name="NOTES"]
struct NoteMtda {
    id: i64,
    tags: Vec<String>,
}

impl From<String> for NoteMtda {
    fn from(src: String) -> Self {
        let mut note: NoteMtda = Default::default();
        let yaml = YamlLoader::load_from_str(&src).unwrap();
        let src = yaml[0].clone();
        if let Yaml::Integer(id) = &src["id"] {
            note.id = *id as i64;
        }
        if let Yaml::Array(elems) = &src["tags"] {
            note.tags = elems
                .to_vec()
                .iter()
                .filter_map(|x| x
                    .clone()
                    .into_string())
                .collect();
        }
        note
    }
}

#[derive(Debug, Insertable, Queryable, AsChangeset)]
#[table_name = "notes"]
struct Note {
    id: i64,
    title: String,
}

#[derive(Debug, Insertable, Queryable, AsChangeset)]
#[table_name = "tags"]
struct Tag {
    noteId: i64,
    tag: String,
}
// Layout:
// Some environment config:
// - contains directory to search watch
// - watch directory for writes
// - on write, or startup read frontmatter and insert into a sqlite DB.
