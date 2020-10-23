use super::metadata::Metadata;
use std::{collections::HashMap, env, ffi::OsStr, path::PathBuf, process::Command};

pub struct Commands<'a> {
    pub datadir: &'a PathBuf,
    pub podir: &'a PathBuf,
    pub metadata: &'a Metadata,
    pub profile: &'a str,
}

impl<'a> Commands<'a> {
    // Detect all .in files in data folder
    // and fill data in templates then move
    // to target/{debug/release}/data directory
    // If file isn't .in move it as it is.
    pub fn process_config_files(&self, outdir: &PathBuf) -> Option<()> {
        if self.datadir.exists() {
            let gresource_id = &self.metadata.id.replace(".", "/");

            let mut variables = HashMap::new();
            variables.insert("@APP_ID@", &self.metadata.id);
            variables.insert("@APP_BINARY@", &self.metadata.bin);
            variables.insert("@APP_NAME@", &self.metadata.name);
            variables.insert("@APP_VERSION@", &self.metadata.version);
            variables.insert("@GRESOURCE_ID@", &gresource_id);
            variables.insert("@GETTEXT_DOMAIN@", &self.metadata.bin);

            std::fs::create_dir_all(&outdir).ok()?;

            for file in std::fs::read_dir(&self.datadir).ok()? {
                let path = file.ok()?.path();
                if path.extension() == Some(OsStr::new("in")) {
                    let mut data = std::fs::read_to_string(&path).ok()?;
                    for (key, value) in variables.iter() {
                        data = data.replace(key, &value);
                    }
                    let output = outdir.join(&path.file_stem()?);
                    std::fs::write(&output.as_path(), data).unwrap();
                } else if path.is_file() {
                    std::fs::copy(&path, outdir.join(&path.file_name()?)).ok()?;
                }
            }
        }

        Some(())
    } //------------------------------------------------------

    // Process and compile po files
    // and move to share/locale/{lang}/LC_MESSAGES/{lang}.mo
    pub fn install_langauge_files(&self, prefix: &PathBuf) -> Option<()> {
        if self.podir.exists() {
            let modir = prefix.join("share/locale");
            std::fs::create_dir_all(&modir).ok()?;
            for file in std::fs::read_dir(&self.podir.as_path()).ok()? {
                let path = file.ok()?.path();

                if path.extension() == Some(OsStr::new("po")) {
                    let name = path.file_stem()?.to_str()?;
                    let modir = modir.join(&name).join("LC_MESSAGES");
                    std::fs::create_dir_all(&modir).ok()?;

                    let mut mo = modir.join(&name);
                    mo.set_extension("mo");

                    Command::new("msgfmt")
                        .args(&[path.to_str()?, "-o"])
                        .arg(&mo)
                        .status()
                        .ok()?;
                }
            }
        }

        Some(())
    } //------------------------------------------------------

    // Translate .appdata.xml and .desktop and
    // install to share/appdata and share/applications
    pub fn install_appdata_and_desktop(
        &self,
        appdata: &PathBuf,
        desktop: &PathBuf,
        prefix: &PathBuf,
    ) -> Option<()> {
        for file in &[appdata, desktop] {
            let path = file.as_path();
            if path.exists() {
                let ttype = "--".to_owned() + path.extension()?.to_str()?;

                let ndir = prefix.join("share").join(if ttype == "--desktop" {
                    "applications"
                } else {
                    "appdata"
                });

                let npath = ndir.join(file.file_name()?);
                std::fs::create_dir_all(&ndir).ok()?;

                if self.podir.exists() {
                    Command::new("msgfmt")
                        .arg(ttype)
                        .args(&["--template", &path.to_str()?])
                        .args(&["-d", &self.podir.as_path().to_str()?])
                        .args(&["-o", &npath.as_path().to_str()?])
                        .status()
                        .ok()?;
                } else {
                    std::fs::copy(&path, &npath.as_path()).ok()?;
                }
            }
        }
        Some(())
    } //------------------------------------------------------

    //Compile glib resources and install it to
    // share/{app_id}/{app_id}.
    pub fn install_glib_resources(&self, glibresource: &PathBuf, prefix: &PathBuf) -> Option<()> {
        let resourcedir = self.datadir.join("resources");
        if glibresource.exists() && resourcedir.exists() {
            let installdir = prefix.join("share").join(&self.metadata.id);
            std::fs::create_dir_all(&installdir).ok()?;

            Command::new("glib-compile-resources")
                .args(&[
                    glibresource.to_str()?,
                    "--sourcedir",
                    resourcedir.to_str()?,
                    "--internal",
                    "--generate",
                    "--target",
                    installdir
                        .join(self.metadata.id.clone() + ".gresource")
                        .to_str()?,
                ])
                .status()
                .ok()?;
        } //-------------------------------------------

        Some(())
    }

    // Install scaleable and symbolic icons
    // to share/icons/{scalable/symbolic}/apps
    // If they are exists in data/icons directory
    pub fn install_icon_files(&self, prefix: &PathBuf) -> Option<()> {
        let scalable = self
            .datadir
            .join("icons")
            .join(self.metadata.id.clone() + ".svg");
        let symbolic = self
            .datadir
            .join("icons")
            .join(self.metadata.id.clone() + "-symbolic.svg");

        if scalable.as_path().exists() && symbolic.as_path().exists() {
            let scalabledir = prefix.join("share/icons/hicolor/scalable/apps");
            let symbolicdir = prefix.join("share/icons/hicolor/symbolic/apps");

            // Swap parent path with new path
            let nscalable = scalabledir.join(scalable.file_name()?);
            let nsymbolic = symbolicdir.join(symbolic.file_name()?);

            for pair in &[(scalable, nscalable), (symbolic, nsymbolic)] {
                std::fs::create_dir_all(pair.1.parent()?).ok()?;
                std::fs::copy(&pair.0, &pair.1).ok()?;
            }
        }

        Some(())
    }

    // Install gschema to share/glib-2.0/schema
    pub fn install_glib_schemas(&self, gschema: &PathBuf, prefix: &PathBuf) -> Option<()> {
        if gschema.exists() {
            let installdir = prefix.join("share/glib-2.0/schemas");
            std::fs::create_dir_all(&installdir).ok()?;
            let ngschema = installdir.join(gschema.file_name()?);
            std::fs::copy(&gschema.as_path(), &ngschema.as_path()).ok()?;
            Command::new("glib-compile-schemas")
                .args(&[installdir.as_path().to_str()?])
                .status()
                .ok()?;
        }
        Some(())
    }

    pub fn generate_config_rs(&self, outdir: &PathBuf, prefix: &PathBuf) -> Option<()> {
        let mut config = format!(
            "pub static APP_ID: &str = \"{}\";
        pub static APP_NAME: &str = \"{}\";
        pub static PROFILE: &str = \"{}\";
        pub static VERSION: &str = \"{}\";
        pub static GETTEXT_PACKAGE: &str = \"{}\";",
            &self.metadata.id,
            &self.metadata.name,
            self.profile,
            &self.metadata.version,
            &self.metadata.bin
        )
        .to_owned();

        let appdatadir = &prefix.join("share").join("appdata");
        if appdatadir.exists() {
            config.push_str(&format!(
                "pub static PKGDATADIR: &str = \"{}\";",
                std::fs::canonicalize(appdatadir).ok()?.as_path().to_str()?
            ));
        }

        let localedir = &prefix.join("share").join("locale");
        if localedir.exists() {
            config.push_str(&format!(
                "pub static LOCALEDIR: &str = \"{}\";",
                std::fs::canonicalize(localedir).ok()?.as_path().to_str()?
            ));
        }
        // Generate config.rs
        std::fs::create_dir_all(&outdir).ok()?;
        let dest_path = &outdir.join("config.rs");
        std::fs::write(&dest_path.as_path(), &config).ok()?;
        env::set_var("CONFIG_PATH", dest_path.as_path().to_str()?);

        Some(())
    }

    // Install binary
    pub fn install_binary(&self, buildflags: &[String], prefix: &PathBuf) -> Option<()> {
        Command::new("cargo")
            .args(&["install", "--force"])
            .args(buildflags)
            .args(&["--path", ".", "--root"])
            .arg(prefix.to_str()?)
            .status()
            .ok()?;
        std::fs::remove_file(prefix.join(".crates2.json").as_path()).ok()?;
        std::fs::remove_file(prefix.join(".crates.toml").as_path()).ok()?;

        Some(())
    }
}
