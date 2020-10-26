## Requirements

- Rust **1.41**+

## Installation

Install `cargo pkg` by running: `cargo install cargo-pkg`.

## Initialize Project

To create new project in GTK4, run `cargo pkg new -id "io.foo.Bar" --name "Foo Bar" foo-bar`

This will create a project with ID `io.foo.Bar` with following structure in `foo-bar` directory.
```
├── Cargo.toml
├── data
│   ├── icons
│   ├── io.foo.Bar.appdata.xml.in
│   ├── io.foo.Bar.desktop.in
│   ├── io.foo.Bar.gresource.xml.in
│   ├── io.foo.Bar.gschema.xml.in
│   └── resources
├── po
│   ├── LINGUAS
│   └── POTFILES.in
└── src
    └── main.rs
```
    
To change application ID in all file names `<NEW_ID>.appdata.xml.in`. and chnage ID in Cargo.toml

```
[package.metadata.pkg]
id = "<NEW_ID>"
name = "Foo Bar"
```

To change package name only change require is `name = "Foo Bar"` in toml.

## Setup Enviornment for GTK4

This section will setup GTK-4 enviroment even your destribution not providing latest GTK4 dependencies. If your distribution provides bleading edge GTK-4 dependencies you can ignore this section.

Install flatpak if not exist. `apt-get` is for deb package manager
`apt-get install flatpak`

Add gnome-nightly repository to flatpak for latest GTK-4 dependencies
`flatpak remote-add --if-not-exists gnome-nightly https://nightly.gnome.org/gnome-nightly.flatpakrepo`

Add flathub repository for rust stable
`flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo`

Install gnome sdk and rust-stable sdk
```
flatpak install --user gnome-nightly org.gnome.Sdk//master -y
flatpak install --user flathub org.freedesktop.Sdk.Extension.rust-stable//20.08 -y
```

To get a shell inside an flatpak’s sandbox
`flatpak run --env=PATH=$PATH:/lib/sdk/rust-stable/bin --share=network --filesystem=$(PWD) --command=sh org.gnome.Sdk//master`


(Now you can go through Installation steps)


## Building & Installing Package

Once your crate has been configured, run `cargo pkg install _build` to build release
targets for your application and install to `_build` directory.

To build and run application `cargo pkg run _build`.

Cargo install flags can be supplied to `cargo pkg run <INSTALL_FLAGS> _build` for example
`cargo pkg run --debug -j 1 _build`.

## License

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    https://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

[//]: # (general links)

[cargo subcommand]: https://github.com/rust-lang/cargo/wiki/Third-party-cargo-subcommands
