use std::{
    collections::HashMap, env, ffi::OsStr, fs::read_to_string, path::PathBuf, process::Command,
};

use toml::{from_str, value::Value};

mod commands;
mod metadata;
use commands::Commands;
use metadata::Metadata;

#[macro_export]
macro_rules! isexists {
    ($str: tt) => {
        if !std::process::Command::new($str).output().is_ok() {
            println!("{} not found!", $str);
            std::process::exit(-1);
        }
    };
}

fn help() {
    println!("Usage: cargo pkg [ACTION] [OPTION] DIR");
    std::process::exit(-1);
}

fn main() {
    // Check build dependent packages exists
    // throw error & exit if not.
    isexists!("msgfmt");
    isexists!("glib-compile-resources");
    isexists!("glib-compile-schemas");

    let args = env::args().filter(|s| s != "pkg").collect::<Vec<_>>();

    if args.get(1) == Some(&"new".to_owned())
        && args.get(2) == Some(&"-id".to_owned())
        && args.get(3).is_some()
        && args.get(4) == Some(&"--name".to_owned())
        && args.get(5).is_some()
        && args.get(6).is_some()
    {
        let chars = args[3].matches('.').count();
        if chars == 2 && args[3].chars().last() != Some('.') {
            Builder::create_project(&args[3], &args[5], &args[6]);
            println!(
                "Created \"{}\" with application id \"{}\"",
                args[5], args[3]
            );
        } else {
            println!("App ID must follow this pattern `io.foo.Bar`");
        }
    } else if (args.get(1) == Some(&"run".to_owned()) || args.get(1) == Some(&"install".to_owned()))
        && args.len() > 2
    {
        let buildflags = &args[2..args.len() - 1];
        // println!("Building with agrs {}", buildflags.join(" "));

        let profile = if buildflags.contains(&"--debug".to_owned()) {
            "debug"
        } else {
            "release"
        };

        let prefix = PathBuf::from(match args.last() {
            Some(f) if !f.contains("--") => f,
            _ => "/usr/local",
        });

        let metadata = Metadata::from("Cargo.toml").expect("Error parsing Cargo.toml");
        let builder = Builder::new(&args[2..args.len() - 1], profile);

        let issuccess = builder.build(&metadata, &prefix);

        if args.get(1) == Some(&"run".to_owned()) && issuccess {
            Command::new(prefix.join("bin").join(&metadata.bin).to_str().unwrap())
                .status()
                .ok();
        }
    } else {
        println!("Invalid arguments");
        std::process::exit(-1);
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

    fn build(&self, metadata: &Metadata, prefix: &PathBuf) -> bool {
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

        if let Some(out) = commands.install_binary(self.buildflags, prefix) {
            if out.code() == Some(101) {
                return false;
            } else {
                return true;
            }
        } else {
            return false;
        }
    }

    pub fn create_project(id: &str, name: &str, bin: &str) -> Option<()> {
        Command::new("cargo")
            .args(&["new", "--bin", &bin])
            .status()
            .ok()?;

        let toml = PathBuf::from(bin).join("Cargo.toml");

        let template = std::fs::read_to_string(&toml).ok()?;
        std::fs::write(
            &toml,
            &format!(
                "{}[dependencies]
log = \"0.4\"
gettext-rs = {{ version = \"0.5\", features = [\"gettext-system\"] }}

[dependencies.gtk]
git = \"https://github.com/gtk-rs/gtk4\"
package = \"gtk4\"

[dependencies.glib]
git = \"https://github.com/gtk-rs/glib\"
features = [\"v2_60\"]

[dependencies.gio]
git = \"https://github.com/gtk-rs/gio\"
features = [\"v2_60\"]

[dependencies.gdk]
git = \"https://github.com/gtk-rs/gdk4\"
package = \"gdk4\"
                \n\n[package.metadata.pkg]\nid = \"{}\"\nname = \"{}\"",
                &template[..template.len() - 15],
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
use gtk::ApplicationWindow;

include!(env!(\"CONFIG_PATH\"));

use std::env::args;

fn main() {
    // Initiialize gtk, gstreamer, and libhandy.
    gtk::init().expect(\"Failed to initialize gtk!\");

    // Setup language / translations
    setlocale(LocaleCategory::LcAll, \"\");
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR);
    textdomain(GETTEXT_PACKAGE);

    // Register resources so we can integrate things like UI files, CSS, and icons
    let res = gio::Resource::load(PKGDATADIR.to_owned() + \"/\" + APP_ID + \".gresource\")
        .expect(\"Could not load resources\");
    gio::resources_register(&res);

    // Set up CSS
    let provider = gtk::CssProvider::new();
    provider.load_from_resource(&(GRESOURCE_ID.to_owned() + \"style.css\"));
    gtk::StyleContext::add_provider_for_display(
        &gdk::Display::get_default().unwrap(),
        &provider,
        600,
    );

    let application =
        gtk::Application::new(Some(APP_ID), Default::default())
            .expect(\"Initialization failed...\");

    application.connect_activate(|app| {
        let builder = gtk::Builder::from_resource(&(GRESOURCE_ID.to_owned() + \"window.ui\"));

        let window: ApplicationWindow = builder.get_object(\"window\").expect(\"Couldn't get window\");
        window.set_application(Some(app));
    
        window.show();
    });

    application.run(&args().collect::<Vec<_>>());
}",
        )
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
        <file compressed=\"true\" preprocess=\"xml-stripblanks\">window.ui</file>
        <file compressed=\"true\" alias=\"style.css\">style.css</file>
    </gresource>
</gresources>",
        )
        .ok()?;

        std::fs::write(
            datadir.join("resources").join("window.ui"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<interface>
    <object class=\"GtkApplicationWindow\" id=\"window\">
    <property name=\"visible\">True</property>
    <property name=\"can_focus\">False</property>
    <property name=\"default_width\">600</property>
    <property name=\"default_height\">400</property>
    <child type=\"titlebar\">
        <object class=\"GtkHeaderBar\" id=\"headerbar\">
        <property name=\"visible\">True</property>
        <property name=\"can_focus\">False</property>
        </object>
    </child>
    <child>
        <object class=\"GtkLabel\" id=\"label\">
        <property name=\"visible\">True</property>
        <property name=\"can_focus\">False</property>
        <property name=\"label\" translatable=\"yes\">Hello world!</property>
        <style>
            <class name=\"title-header\"/>
        </style>
        </object>
    </child>
    </object>
</interface>",
        )
        .ok()?;

        std::fs::write(
            datadir.join("resources").join("style.css"),
            ".title-header { font-size: 40px }",
        )
        .ok()?;

        std::fs::write(podir.join("LINGUAS"), "").ok()?;
        std::fs::write(podir.join("POTFILES.in"), "src/main.rs").ok()?;

        Some(())
    }
}
