use std::{
    collections::HashMap, env, ffi::OsStr, fs::read_to_string, path::PathBuf, process::Command,
};
use toml::{from_str, value::Value};

#[macro_export]
macro_rules! die {
    ($fmt:expr) => {
        print!($fmt);
        std::process::exit(-1)
    };
}

mod commands;
mod metadata;
use commands::Commands;
use metadata::Metadata;

fn main() {
    // Parse build args
    let mut args: Vec<_> = env::args().filter(|s| s != "pkg").collect();
    args.drain(..1);

    // Determine profile from build flag
    let profile = if args.contains(&"--release".to_string()) {
        "release"
    } else {
        "debug"
    };

    let prefix = PathBuf::from(match args.last() {
        Some(f) if !f.contains("--") => f,
        _ => "/usr/local",
    });

    if let Some(command) = args.iter().nth(0) {
        match command.as_str() {
            "install" | "run" => {
                if &args.len() > &2 {
                    let metadata = Metadata::from("Cargo.toml").expect("Error parsing Cargo.toml");
                    let builder = Builder::new(&args[1..args.len() - 1], profile);

                    println!("\x1b[1;38;5;29m   Compiling\x1b[0m {}", metadata.id);
                    builder.build(&metadata, &prefix);

                    if command.as_str() == "run" {
                        Command::new(prefix.join("bin").join(&metadata.bin).to_str().unwrap())
                            .status()
                            .ok();
                    }
                } else {
                    println!("Invalid arguments");
                    std::process::exit(-1);
                }
            }
            "new" => {
                Builder::create_project("io.foo.Bar", "Foo Bar", "foo-bar")
                    .expect("Error creating project");
            }
            _ => {
                println!("Invalid arguments");
                std::process::exit(-1);
            }
        }
    }
}

struct Builder<'a> {
    // Profile can be either "release" or "debug"
    profile: &'a str,
    buildflags: &'a [String],
}

impl<'a> Builder<'a> {
    fn new(buildflags: &'a [String], profile: &'a str) -> Self {
        Builder {
            buildflags,
            profile,
        }
    }

    fn build(&self, metadata: &Metadata, prefix: &PathBuf) {

        if !std::process::Command::new("msgfmt").spawn().is_ok() {
            println!("msgfmt not found!");
            std::process::exit(-1);
        }
        if !std::process::Command::new("glib-compile-resources").spawn().is_ok() {
            println!("glib-compile-resources not found!");
            std::process::exit(-1);
        }
        if !std::process::Command::new("glib-compile-schemas").spawn().is_ok() {
            println!("glib-compile-schemas not found!");
            std::process::exit(-1);
        }
        
        let datadir = PathBuf::from("data");
        let podir = PathBuf::from("po");

        let outdir = metadata.targetdir.join(self.profile).join("data");
        std::fs::create_dir_all(&outdir).expect(&format!(
            "Error creating target/{}/data directory",
            self.profile
        ));

        let commands = Commands {
            datadir: &datadir,
            podir: &podir,
            metadata: &metadata,
            profile: &self.profile,
        };

        println!("\x1b[1;38;5;29m  Processing\x1b[0m .in files");
        commands
            .process_config_files(&outdir)
            .expect("Error processing .in files");

        println!("\x1b[1;38;5;29m   Compiling\x1b[0m langauge files");
        commands
            .install_langauge_files(&prefix)
            .expect("Error compiling langage files");

        //---------------------------------------------------------------
        println!("\x1b[1;38;5;29m  Generating\x1b[0m appdata and desktop files");
        let appdata = outdir.join(metadata.id.clone() + ".appdata.xml");
        let desktop = outdir.join(metadata.id.clone() + ".desktop");
        commands
            .install_appdata_and_desktop(&appdata, &desktop, prefix)
            .expect("Error installing appdata and desktop");
        //-----------------------------------------------------------

        //-----------------------------------------------------------
        println!("\x1b[1;38;5;29m  Installing\x1b[0m glib resources");
        let glibresource = outdir.join(metadata.id.clone() + ".gresource.xml");
        commands
            .install_glib_resources(&glibresource, prefix)
            .expect("Error compiling and installing glib resources");
        //-----------------------------------------------------------

        //-----------------------------------------------------------
        println!("\x1b[1;38;5;29m  Installing\x1b[0m icon files");
        commands
            .install_icon_files(prefix)
            .expect("Error installing icons");
        //-----------------------------------------------------------

        //-----------------------------------------------------------
        println!("\x1b[1;38;5;29m  Installing\x1b[0m glib schemas");
        let gschema = datadir.join(metadata.id.clone() + ".gschema.xml");
        commands
            .install_glib_schemas(&gschema, prefix)
            .expect("Error installing glib schemas");
        //-----------------------------------------------------------

        //-----------------------------------------------------------
        println!("\x1b[1;38;5;29m  Generating\x1b[0m config.rs file");
        commands
            .generate_config_rs(&outdir, prefix)
            .expect("Error generating config.rs");
        //-----------------------------------------------------------

        commands
            .install_binary(self.buildflags, prefix)
            .expect("Error installing binary");
    }

    pub fn create_project(id: &str, name: &str, bin: &str) -> Option<()> {
        Command::new("cargo")
            .args(&["new", "--bin", &bin])
            .status()
            .ok()?;

        let toml = PathBuf::from(bin).join("Cargo.toml");

        std::fs::write(
            &toml,
            &format!(
                "{}gtk  = {{ git = \"https://github.com/gtk-rs/gtk\",  features = [\"v3_24\"] }}
gdk  = {{ git = \"https://github.com/gtk-rs/gdk\",  features = [\"v3_24\"] }}
gio  = {{ git = \"https://github.com/gtk-rs/gio\",  features = [\"v2_60\"] }}
glib = {{ git = \"https://github.com/gtk-rs/glib\", features = [\"v2_60\"] }}

libhandy = {{ git = \"https://gitlab.gnome.org/kavanmevada/libhandy-rs\", branch=\"devel\" }}
gettext-rs = {{ version = \"0.4.4\" , features = [\"gettext-system\"] }}\n\n[package.metadata.pkg]\nid = \"{}\"\nname = \"{}\"",
                std::fs::read_to_string(&toml).ok()?,
                id,
                name
            ),
        )
        .ok()?;

        std::fs::write(
            &PathBuf::from(bin).join("src").join("main.rs"),
            "use gettextrs::*;
extern crate gio;
extern crate gtk;

use gio::prelude::*;
use gtk::prelude::*;

include!(env!(\"CONFIG_PATH\"));

use std::env::args;

fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);

    window.set_title(APP_NAME);
    window.set_border_width(10);
    window.set_position(gtk::WindowPosition::Center);
    window.set_default_size(350, 70);

    let button = gtk::Button::with_label(\"Click me!\");

    window.add(&button);

    window.show_all();
}

fn main() {
    // Setup language / translations
    setlocale(LocaleCategory::LcAll, \"\");
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR);
    textdomain(GETTEXT_PACKAGE);

    // Register resources so we can integrate things like UI files, CSS, and icons
    let res = gio::Resource::load(PKGDATADIR.to_owned() + \"/\" + APP_ID + \".gresource\")
        .expect(\"Could not load resources\");
    gio::resources_register(&res);

    let application =
        gtk::Application::new(Some(APP_ID), Default::default())
            .expect(\"Initialization failed...\");

    application.connect_activate(|app| {
        build_ui(app);
    });

    application.run(&args().collect::<Vec<_>>());
}")
        .ok()?;

        let datadir = PathBuf::from(bin.to_string() + "/data");
        std::fs::create_dir_all(&datadir.join("resources")).ok()?;
        std::fs::create_dir_all(&datadir.join("icons")).ok()?;

        let podir = PathBuf::from(bin.to_string() + "/po");
        std::fs::create_dir_all(&podir).ok()?;

        std::fs::write(
            datadir.join(id.to_owned() + ".gschema.xml.in"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<schemalist gettext-domain=\"@GETTEXT_DOMAIN@\">
    <schema id=\"@APP_ID@\" path=\"/@GRESOURCE_ID@/\">
    </schema>
</schemalist>",
        )
        .ok()?;

        std::fs::write(
            datadir.join(id.to_owned() + ".desktop.in"),
            "[Desktop Entry]
Name=@APP_NAME@
Exec=@APP_BINARY@
Icon=@APP_ID@ // Do not translate
Terminal=false
Type=Application
StartupNotify=true",
        )
        .ok()?;

        std::fs::write(
            datadir.join(id.to_owned() + ".appdata.xml.in"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<component>
    <id>@APP_ID@</id>
    <name>@APP_NAME@</name>
    <summary>A foo-ish bar</summary>
    <url type=\"homepage\">http://www.example.org</url>
    <metadata_license>CC0-1.0</metadata_license>
    <provides>
        <binary>@APP_BINARY@</binary>
    </provides>
    <releases>
        <release version=\"@APP_VERSION@\"/>
    </releases>
    <developer_name>FooBar Team</developer_name>
</component>",
        )
        .ok()?;

        std::fs::write(
            datadir.join(id.to_owned() + ".gresource.xml.in"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<gresources>
    <gresource prefix=\"/@GRESOURCE_ID@/\">
    </gresource>
</gresources>",
        )
        .ok()?;

        std::fs::write(podir.join("LINGUAS"), "").ok()?;
        std::fs::write(podir.join("POTFILES.in"), "src/main.rs").ok()?;

        Some(())
    }
}
