use json::{parse, JsonValue};
use std:: {
    process::Command,
    collections::HashMap,
    path::PathBuf,
    ffi::OsStr,
    env,
    error::Error,
    path::Path
};


#[derive(Debug, Eq, PartialEq)]
enum Profile { Debug, Release }


fn main() {
    let args = cli::parse_varargs();

    let mut prefix = PathBuf::from("usr").join("local");
    if args.contains_key("prefix") {
        prefix = PathBuf::from(&args["prefix"]);
    }

    let mut profile = Profile::Debug;
    if args.contains_key("release") {
        profile = Profile::Release;
    }

    let metadata = Metadata::metadata("Cargo.toml");
    if args.contains_key("install") {
        build_and_install(&metadata, &prefix, &profile);
    } else if args.contains_key("run") {
        build_and_install(&metadata, &prefix, &profile);
        Command::new(prefix.join("bin").join(&metadata.bin).as_os_str().to_str().unwrap())
            .status().expect("Error running binary");
    }

}




fn build_and_install(metadata: &Metadata, prefix: &PathBuf, profile: &Profile) {

    let mut outputdir = metadata.target_dir.join("debug");
    if profile == &Profile::Release {
        outputdir = metadata.target_dir.join("release");
    }

    let pkgdatadir = outputdir.join("data");


    let podir = PathBuf::from("po");
    let datadir = PathBuf::from("data");
    let resourcedir = datadir.join("resources");
    let icondir = datadir.join("icons");


    let appdata = [&metadata.app_id, "appdata.xml"].join(".");
    let gschema = [&metadata.app_id, "gschema.xml"].join(".");
    let gresource = [&metadata.app_id, "gresource.xml"].join(".");
    let desktop = [&metadata.app_id, "desktop"].join(".");
    let resource = [&metadata.app_id, "gresource"].join(".");

    let scalable = [&metadata.app_id, "svg"].join(".");
    let symbolic = [&metadata.app_id, "symbolic.svg"].join("-");


    let bindir = prefix.join("bin");
    let sharedir = prefix.join("share");
    let appdatadir = sharedir.join("appdata");
    let applicationsdir = sharedir.join("applications");
    let localedir = sharedir.join("locale");


    let mut map = HashMap::new();
    map.insert("@APP_ID@", &metadata.app_id);
    map.insert("@APP_NAME@", &metadata.name);
    map.insert("@APP_VERSION@", &metadata.version);
    map.insert("@GETTEXT_DOMAIN@", &metadata.gettextdomain);




    let icons = Icon {
        scalable: &icondir.join(&scalable),
        symbolic: &icondir.join(&symbolic),
    }.install_all(prefix);

    glib_compile_resources(
        &pkgdatadir.join(&gresource),
        resourcedir,
        &prefix
    );

    configuration_data_dir(&datadir, &pkgdatadir, &map);

    if podir.exists() {
        if podir.join("POTFILES.in").exists()
            && podir.join("LINGUAS").exists() {
            msgfmt(pkgdatadir.join(&desktop), "--desktop", &applicationsdir, &podir);
            msgfmt(pkgdatadir.join(&appdata), "--xml", &appdatadir, &podir);
        }

        std::fs::create_dir_all(&localedir);
        let paths = std::fs::read_dir(&podir.as_path().display().to_string())
            .expect("Error reading po directory");

        for path in paths {
            let mut p = path.unwrap().path();
            let mut n = p.strip_prefix("po").unwrap();

            if n.extension() == Some(OsStr::new("po")) {
                let name = p.file_stem().unwrap().to_os_string();
                let modir = localedir.join(&name).join("LC_MESSAGES");
                let mut mo = modir.join(&name);
                mo.set_extension("mo");

                std::fs::create_dir_all(&modir);
                Command::new("msgfmt").args(&[&p.display().to_string(), "-o"])
                    .arg(&mo).status().expect("Error executing msgfmt");
            }
        }
    }



    std::fs::create_dir_all(&pkgdatadir);
    let dest_path = PathBuf::from(&pkgdatadir).join("config.rs");
    std::fs::write(&dest_path.as_path(),
        &format!(
            "pub static APP_ID: &str = \"{}\";
pub static APP_NAME: &str = \"{}\";
pub static PROFILE: &str = \"{}\";
pub static VERSION: &str = \"{}\";
pub static GETTEXT_PACKAGE: &str = \"{}\";
pub static PKGDATADIR: &str = \"{}\";
pub static LOCALEDIR: &str = \"{}\";
",
            &metadata.app_id,
            &metadata.name,
            if profile == &Profile::Release { "release" } else { "debug" },
            &metadata.version,
            &metadata.gettextdomain,
            std::fs::canonicalize(&appdatadir).unwrap().display().to_string(),
            std::fs::canonicalize(&localedir).unwrap().display().to_string(),
        )
    ).unwrap();
    env::set_var("CONFIG_PATH", dest_path.as_path().display().to_string());


    Cargo::install(prefix, 
        if *profile == Profile::Debug { &["--debug"] }
        else { &[] }
    );
}



fn msgfmt(template: PathBuf, template_type: &str, installdir: &PathBuf, podir: &PathBuf) -> Option<std::process::ExitStatus> {
    std::fs::create_dir_all(&installdir);
    let outpath = template.swap_parent(installdir)?;

    let status = Command::new("msgfmt").args(&[
        template_type,
        "--template",
        &template.into_os_string().to_str()?,
        "-d", &podir.to_owned().into_os_string().to_str()?,
        "-o", &outpath.into_os_string().to_str()?,
    ]).status().ok()?;

    Some(status)
}


fn configuration_data_dir(datadir: &PathBuf, export: &PathBuf, variables: &HashMap<&str, &String>) -> Option<bool> {
    std::fs::create_dir_all(&export);
    for file in std::fs::read_dir(&datadir.as_path()).ok()? {
        let mut p = file.ok()?.path();
        if p.extension() == Some(OsStr::new("in")) {
            let mut data = std::fs::read_to_string(&p).ok()?;
            for (key, value) in variables.iter() {   
                data = data.replace(key, &value);
            };
            let output = export.join(&p.file_stem()?);
            std::fs::write(&output.as_path(), data).unwrap();
        }
    }
    Some(true)
}


fn glib_compile_resources(path: &PathBuf, resourcedir: PathBuf, prefix: &PathBuf) -> Option<std::process::ExitStatus> {
    let fname = &path.file_name()?.to_str()?;
    let installdir = prefix.join("share")
        .join("glib-2.0").join("schemas");

    std::fs::create_dir_all(&installdir);
    
    let status = Command::new("glib-compile-resources").args(&[
        &path.to_owned().into_os_string().to_str()?,
        "--sourcedir",
        &resourcedir.into_os_string().to_str()?,
        "--internal",
        "--generate",
        "--target",
        &installdir.join(&fname.replace(".xml", "")).into_os_string().to_str()?,
    ]).status().ok()?;

    Some(status)
}


mod Cargo {
    use std::fs::remove_file;
    use std::process::Command;
    use std::path::PathBuf;
    use std::error::Error;

    pub fn install(prefix: &PathBuf, flags: &[&str]) -> Option<std::process::ExitStatus> {
        let prefix_os_str = prefix.as_os_str().to_str();

        let result = Command::new("cargo").args(&["install", "--force"])
            .args(flags).args(&["--path", ".", "--root"])
            .arg(prefix_os_str?)
            .status().ok()?;

        remove_file(prefix.join(".crates2.json").as_path());
        remove_file(prefix.join(".crates.toml").as_path());

        Some(result)
    }
}



trait Files {
    fn swap_parent(&self, parent: &PathBuf) -> Option<PathBuf>;
}

impl Files for PathBuf {
    fn swap_parent(&self, parent: &PathBuf) -> Option<PathBuf> {
        Some(parent.join(self.file_name()?))
    }
}


struct Icon<'a> {
    scalable: &'a PathBuf,
    symbolic: &'a PathBuf
}

impl Icon<'_> {
    fn install_all(&self, prefix: &PathBuf) {

        let scalabledir = prefix.join("share/icons/scalable/apps");
        let symbolicdir = prefix.join("share/icons/symbolic/apps");

        if let (Some(scalable), Some(symbolic)) = (
            &self.scalable.swap_parent(&scalabledir),
            &self.symbolic.swap_parent(&symbolicdir)
        ) {
            std::fs::create_dir_all(scalabledir.as_path());
            std::fs::copy(self.scalable, scalable);

            std::fs::create_dir_all(symbolicdir.as_path());
            std::fs::copy(self.symbolic, symbolic);
        }
    }
}






#[derive(Debug, Clone)]
struct Metadata {
    app_id: String,
    bin: String,
    name: String,
    version: String,
    gettextdomain: String,
    target_dir: PathBuf,
}

impl Metadata {
    fn metadata(toml_path: &str) -> Self {
        let output = String::from_utf8(
            Command::new("cargo")
                .arg("metadata")
                .arg("--format-version=1")
                .arg(format!("--manifest-path={}", toml_path))
                .output()
                .expect("Error executing cargo metadata command")
                .stdout
        ).expect("Error parsing command output to String");
    
        let j = json::parse(&output).expect("Error parsing JSON Object");

        let root = j["resolve"]["root"].to_owned().as_str().map(str::to_string)
            .expect("Root package not found");

        let target = j["target_directory"].to_owned().as_str().map(str::to_string)
            .expect("Target directory not found");

        let root_vec: Vec<_> = root.split(' ').collect();


        let packages = j["packages"].to_owned();
        let members = packages.members().filter(|p| p["name"] == root_vec[0]).collect::<Vec<_>>();
        let selected = members[0];
        
        let custom_meta = selected["metadata"]["pkg"].to_owned();

        if custom_meta == json::JsonValue::Null {
            println!("No [package.metadata.pkg] in Cargo.toml!");
            std::process::exit(1);
        }

        
        let entries: Vec<_> = custom_meta.entries()
            .map(|(k, v)| (k, v.as_str()))
            .filter(|(k, v)| v.is_some())
            .map(|(k, v)| (k, v.unwrap()))
            .collect::<Vec<_>>();
        
        let mapped_entries: HashMap<&str, &str> = entries.into_iter().collect();

        if !mapped_entries.contains_key("id")
            && !mapped_entries.contains_key("name") {
                println!("No [package.metadata.pkg.id/name] in Cargo.toml!");
                std::process::exit(1);
        }


        Self {
            app_id: mapped_entries["id"].to_string(),
            bin: root_vec[0].to_string(),
            name: mapped_entries["name"].to_string(),
            version: root_vec[1].to_string(),
            gettextdomain: root_vec[0].to_string(),
            target_dir: PathBuf::from(target)
        }
    }
    
}


mod cli {
    use std::collections::HashMap;
    use std::env;

    // Parse CLI arguments
    //---------------------------
    const DEFINED_ARGS: [(&str, bool); 4] = [
        ("install" , false),
        ("run" , false),
        ("prefix"  , true),
        ("release" , false)
    ];

    pub fn parse_varargs<'a>() -> HashMap<&'a str, String> {
        let args: Vec<String> = env::args().collect();
        let mut parsed_map: HashMap<&str, String> = HashMap::new();

        for param in args {
            for arg in DEFINED_ARGS.iter() {
                if param[2..].starts_with(arg.0)  {
                    if arg.1 == false {
                        &parsed_map
                            .insert(arg.0, "true".to_string());
                    } else if param.contains('=') {
                        &parsed_map
                            .insert(arg.0, param[arg.0.len()+3..].to_string());
                    } else {
                        println!("Error parsing argument!");
                        std::process::exit(1)
                    }
                }
            }
        }

        if parsed_map.len() == 0 {
            println!("No argument passed!");
            std::process::exit(1)
        }

        parsed_map
    }
}